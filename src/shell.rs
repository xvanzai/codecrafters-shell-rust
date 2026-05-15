use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Cursor, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use rustyline::history::History;

use crate::builtins::jobs::Job;
use crate::builtins::{self, Builtin, ShouldExit};
use crate::completer::{ShellHelper, create_editor_with_helper};
use crate::context::ShellContext;
use crate::error::ShellError;
use crate::parser::{self, ParsedCommand, Pipeline, Redirection};

pub struct Shell {
    builtins: HashMap<String, Box<dyn Builtin>>,
    context: ShellContext,
    editor: rustyline::Editor<ShellHelper, rustyline::history::FileHistory>,
}

impl Shell {
    pub fn new() -> Self {
        let mut context = ShellContext::new();
        let mut builtins: HashMap<String, Box<dyn Builtin>> = HashMap::new();

        // 注册所有内建命令，并同步内建名称到 context
        let cmd_list: Vec<Box<dyn Builtin>> = vec![
            Box::new(builtins::ExitBuiltin),
            Box::new(builtins::EchoBuiltin),
            Box::new(builtins::TypeBuiltin),
            Box::new(builtins::PwdBuiltin),
            Box::new(builtins::CdBuiltin),
            Box::new(builtins::CompleteBuiltin),
            Box::new(builtins::JobsBuiltin),
            Box::new(builtins::HistoryBuiltin),
        ];

        for builtin in cmd_list {
            let name = builtin.name();
            context.register_builtin_name(name);
            builtins.insert(name.to_string(), builtin);
        }

        // 创建编辑器并绑定补全器
        let editor = create_editor_with_helper(&context);
        // 加载历史文件（忽略错误）
        // let _ = editor.load_history(".shell_history");

        Shell {
            builtins,
            context,
            editor,
        }
    }

    pub fn run(&mut self) -> Result<(), ShellError> {
        loop {
            self.context
                .print_background_jobs_is_done(&mut io::stdout())?;

            let readline = self.editor.readline("$ ");

            match readline {
                Ok(line) => {
                    // 处理用户输入
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    // 立即添加到 rustyline 的内存历史中
                    let _ = self.editor.add_history_entry(trimmed);
                    // 同步历史记录（含当前输入）
                    self.context.history_entries = self
                        .editor
                        .history()
                        .iter()
                        .map(|s| s.to_string())
                        .collect();

                    // 原有输入获取与 trimming 后...
                    let pipeline = if trimmed.contains('|') {
                        parser::parse_pipeline(trimmed)?
                    } else {
                        // 单命令（可能含 &、重定向、内建命令等）
                        let cmd = parser::parse(trimmed)?; // 原有 parse 函数完全不变
                        Pipeline {
                            commands: vec![cmd],
                        }
                    };

                    // 统一通过 Pipeline 的长度选择执行路径
                    if pipeline.commands.len() == 1 {
                        // 单命令：走原有 execute_command（支持内建、后台、重定向等）
                        match self.execute_command(pipeline.commands.into_iter().next().unwrap()) {
                            Ok(ShouldExit::Continue) => {}
                            Ok(ShouldExit::Exit) => break,
                            Err(e) => eprintln!("{}", e),
                        }
                    } else {
                        // 管道：走新方法
                        if let Err(e) = self.execute_pipeline(pipeline) {
                            eprintln!("{}", e);
                        }
                    }
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    break; // 用户按下 Ctrl+D
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    continue; // 用户按下 Ctrl+C
                }
                Err(e) => {
                    eprintln!("Error reading line: {}", e);
                    break;
                }
            }

            // 处理历史清除请求（由 history -c 触发）
            if self.context.request_clear_history {
                let _ = self.editor.history_mut().clear();
                self.context.history_entries.clear();
                self.context.request_clear_history = false;
            }

            // 处理历史加载请求
            if let Some(ref file) = self.context.request_load_history.take() {
                self.context.history_entries.clear(); // 清空当前历史记录
                let _ = self.editor.history_mut().append(Path::new(file)); // 加载文件内容到内存历史
                // 同步到 context.history_entries
                self.context.history_entries = self
                    .editor
                    .history()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
            }

            // let _ = self.editor.save_history(".shell_history"); // 每次循环结束时保存历史，确保持久化最新记录
        }
        Ok(())
    }

    fn execute_command(&mut self, cmd: ParsedCommand) -> Result<ShouldExit, ShellError> {
        let ParsedCommand {
            name,
            args,
            redirects,
            is_background,
        } = cmd;

        // 1. 分离 stdout / stderr 重定向
        let stdout_redirs: Vec<_> = redirects
            .iter()
            .filter(|r| matches!(r, Redirection::Overwrite(_) | Redirection::Append(_)))
            .collect();
        let stderr_redirs: Vec<_> = redirects
            .iter()
            .filter(|r| {
                matches!(
                    r,
                    Redirection::StderrOverwrite(_) | Redirection::StderrAppend(_)
                )
            })
            .collect();

        // 2. 产生副作用并获取最终句柄
        let mut final_stdout = open_redirect_chain(&stdout_redirs)?;
        let mut final_stderr = open_redirect_chain(&stderr_redirs)?;

        // 3. 动态分发 writer
        let output: &mut dyn Write = match &mut final_stdout {
            Some(f) => f,
            None => &mut io::stdout(),
        };
        let error_output: &mut dyn Write = match &mut final_stderr {
            Some(f) => f,
            None => &mut io::stderr(),
        };

        // 4. 内建命令
        if let Some(builtin) = self.builtins.get(&name) {
            return match builtin.execute(&args, &mut self.context, output) {
                Ok(exit_code) => Ok(exit_code),
                Err(e) => {
                    // 内建命令的错误写入其 stderr 流（已重定向或终端）
                    // 忽略写入错误（文件可能被删除等）
                    let _ = writeln!(error_output, "{}", e);
                    Ok(ShouldExit::Continue)
                }
            };
        }

        // 5. 外部命令
        let path = self
            .context
            .resolve_cmd(&name)
            .ok_or_else(|| ShellError::CommandNotFound(name.clone()))?;

        let mut command = Command::new(path.file_name().unwrap());
        command.args(&args);

        // 将文件句柄转移给 Command（注意：此时 final_stdout/stderr 已不再被借用）
        if let Some(file) = final_stdout.take() {
            command.stdout(file);
        }
        if let Some(file) = final_stderr.take() {
            command.stderr(file);
        }

        if is_background {
            // 后台运行：spawn 并存储句柄，不等待
            match command.spawn() {
                Ok(child) => {
                    // 打印作业信息，例如 [1] 12345
                    println!(
                        "[{}] {}",
                        self.context.background_jobs.len() + 1,
                        child.id()
                    );
                    self.context.add_background_job(Job {
                        id: self.context.background_jobs.len() + 1,
                        command: format!("{} {}", name, args.join(" ")),
                        child,
                    });
                    Ok(ShouldExit::Continue)
                }
                Err(e) => Err(ShellError::Io(e)),
            }
        } else {
            // 前台运行：等待结束
            let _status = command.status().map_err(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    ShellError::CommandNotFound(name.clone())
                } else {
                    ShellError::Io(e)
                }
            })?;
            Ok(ShouldExit::Continue)
        }
    }

    fn execute_pipeline(&mut self, pipeline: Pipeline) -> Result<ShouldExit, ShellError> {
        let mut children: Vec<Child> = Vec::new();
        let mut previous_stdout: Option<std::process::ChildStdout> = None;
        let mut previous_output: Option<Vec<u8>> = None;
        let num = pipeline.commands.len();

        for (i, cmd) in pipeline.commands.iter().enumerate() {
            let ParsedCommand {
                name,
                args,
                redirects,
                is_background: _,
            } = cmd;

            let is_builtin = self.builtins.contains_key(name);
            let is_last = i == num - 1;

            // ─── 内建命令处理 ─────────────────────────────
            if is_builtin {
                let builtin = self.builtins.get(name).unwrap();

                // 需要输入的内建命令暂不支持放入管道
                if builtin.needs_stdin() {
                    return Err(ShellError::BuiltinError(format!(
                        "{}: cannot pipe input to builtin command",
                        name
                    )));
                }

                // 处理 stdout 重定向：检查该命令是否有 > 或 >>
                let stdout_redir = redirects
                    .iter()
                    .any(|r| matches!(r, Redirection::Overwrite(_) | Redirection::Append(_)));

                // 捕获内建命令的标准输出
                let mut output_buf = Cursor::new(Vec::new());
                match builtin.execute(args, &mut self.context, &mut output_buf) {
                    Ok(ShouldExit::Continue) => {}
                    Ok(ShouldExit::Exit) => {
                        return Ok(ShouldExit::Exit);
                    }
                    Err(e) => {
                        // 错误写入终端 stderr（与 execute_command 行为一致）
                        // 暂不考虑管道中内建命令的 stderr 重定向
                        eprintln!("{}", e);
                    }
                }

                let data = output_buf.into_inner();

                if stdout_redir {
                    // 应用重定向：将数据写入文件，不再映射到 previous_output
                    if let Err(e) = apply_builtin_stdout_redirect(&data, redirects) {
                        eprintln!("{}", e);
                    }
                    // 有重定向后，数据不向后传递，后续命令若无其他输入则继承终端
                } else if is_last {
                    io::stdout().write_all(&data)?;
                } else {
                    // 非最后一个且无重定向：暂存数据，供下一个命令（应为外部命令）使用
                    previous_output = Some(data);
                }

                continue;
            }

            // ─── 外部命令处理 ─────────────────────────────
            // 1. 路径查找（复用 ShellContext 缓存）
            let path = self
                .context
                .resolve_cmd(name)
                .ok_or_else(|| ShellError::CommandNotFound(name.clone()))?;

            let mut command = Command::new(&path);
            command.args(args);

            // 2. 处理 stdout 重定向（复用 execute_command 中的 apply_stdout_redirect）
            let stdout_redir = redirects
                .iter()
                .any(|r| matches!(r, Redirection::Overwrite(_) | Redirection::Append(_)));
            if is_last {
                if stdout_redir {
                    apply_stdout_redirect(&mut command, redirects)?;
                }
                // 否则 stdout 默认 inherit
            } else {
                if stdout_redir {
                    apply_stdout_redirect(&mut command, redirects)?;
                } else {
                    command.stdout(Stdio::piped());
                }
            }

            // 3. 处理 stdin
            //    考虑三种可能：① 第一个命令  ② 前一个外部命令有管道  ③ 前一个内建命令有数据
            let prev_data = previous_output.take();
            if i == 0 {
                command.stdin(Stdio::inherit());
            } else if prev_data.is_some() {
                // 前一个命令是内建命令：需要写入数据，先设 stdin 为 piped
                command.stdin(Stdio::piped());
            } else if let Some(prev_stdout) = previous_stdout.take() {
                // 前一个命令是外部命令且有管道 stdout
                command.stdin(prev_stdout);
            } else {
                command.stdin(Stdio::inherit());
            }

            // stderr 保持继承
            command.stderr(Stdio::inherit());

            // 4. 启动子进程
            let mut child = command.spawn().map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    ShellError::CommandNotFound(name.clone())
                } else {
                    ShellError::Io(e)
                }
            })?;

            // 5. 如果有来自内建命令的数据，写入子进程的 stdin 并关闭
            if let Some(data) = prev_data
                && let Some(mut stdin) = child.stdin.take()
            {
                stdin.write_all(&data)?;
                // stdin 自动 drop，关闭管道
            }

            // 6. 如果不是最后一个命令且 stdout 未被重定向，保存 stdout 管道供下一个命令使用
            if !is_last && !stdout_redir {
                previous_stdout = child.stdout.take();
            }

            children.push(child);
        }

        // 等待所有子进程结束
        for mut child in children {
            let _ = child.wait();
        }

        Ok(ShouldExit::Continue)
    }
}

// ── 辅助函数 ────────────────────────────────────────────

/// 依次打开/创建重定向文件（产生截断/追加副作用），返回最后一个句柄。
fn open_redirect_chain(redirs: &[&Redirection]) -> Result<Option<File>, ShellError> {
    let mut last = None;
    for redir in redirs {
        last = Some(open_redirect_file(redir)?);
    }
    Ok(last)
}

fn open_redirect_file(redir: &Redirection) -> Result<File, ShellError> {
    let (filename, append) = match redir {
        Redirection::Overwrite(f) | Redirection::StderrOverwrite(f) => (f, false),
        Redirection::Append(f) | Redirection::StderrAppend(f) => (f, true),
    };

    let file = if append {
        OpenOptions::new().create(true).append(true).open(filename)
    } else {
        File::create(filename)
    };

    file.map_err(|e| ShellError::BuiltinError(format!("failed to open {}: {}", filename, e)))
}

fn apply_stdout_redirect(
    command: &mut Command,
    redirects: &[Redirection],
) -> Result<(), ShellError> {
    for redir in redirects
        .iter()
        .filter(|r| matches!(r, Redirection::Overwrite(_) | Redirection::Append(_)))
    {
        let (file, append) = match redir {
            Redirection::Overwrite(f) => (f, false),
            Redirection::Append(f) => (f, true),
            _ => continue,
        };
        let file = if append {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(file)
        } else {
            std::fs::File::create(file)
        }
        .map_err(|_e| ShellError::BuiltinError(format!("{}: cannot open", file)))?;
        command.stdout(file);
        break; // 只应用最后一个 stdout 重定向（POSIX 语义）
    }
    Ok(())
}

/// 辅助函数：将内建命令的输出应用重定向写入文件
fn apply_builtin_stdout_redirect(data: &[u8], redirects: &[Redirection]) -> Result<(), ShellError> {
    let target = redirects
        .iter()
        .rev()
        .find(|r| matches!(r, Redirection::Overwrite(_) | Redirection::Append(_)));
    if let Some(redir) = target {
        let (filename, append) = match redir {
            Redirection::Overwrite(f) => (f, false),
            Redirection::Append(f) => (f, true),
            _ => unreachable!(),
        };
        let mut file = if append {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(filename)
        } else {
            std::fs::File::create(filename)
        }
        .map_err(|e| ShellError::BuiltinError(format!("{}: cannot open: {}", filename, e)))?;
        file.write_all(data)?;
    }
    Ok(())
}
