use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::process::{Child, Command, Stdio};

use crate::builtins::jobs::Job;
use crate::builtins::{self, Builtin, ShouldExit};
use crate::completer::{ShellHelper, create_editor_with_helper};
use crate::context::ShellContext;
use crate::error::ShellError;
use crate::parser::{self, ParsedCommand, Pipeline, Redirection};

pub struct Shell {
    builtins: HashMap<String, Box<dyn Builtin>>,
    context: ShellContext,
    editor: rustyline::Editor<ShellHelper, rustyline::history::DefaultHistory>,
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
        ];

        for builtin in cmd_list {
            let name = builtin.name();
            context.register_builtin_name(name);
            builtins.insert(name.to_string(), builtin);
        }

        // 创建编辑器并绑定补全器
        let editor = create_editor_with_helper(&context);

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

        let cmd_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ShellError::CommandNotFound(name.clone()))?;

        let mut command = Command::new(cmd_name);
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
                        command: format!("{} {}", cmd_name, args.join(" ")),
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

        let num = pipeline.commands.len();

        for (i, cmd) in pipeline.commands.iter().enumerate() {
            let ParsedCommand {
                name,
                args,
                redirects,
                is_background: _,
            } = cmd;

            let path = self
                .context
                .resolve_cmd(name)
                .ok_or_else(|| ShellError::CommandNotFound(name.clone()))?;

            let mut command = Command::new(&path);
            command.args(args);

            // ----- 处理标准输出 -----
            let stdout_redir = redirects
                .iter()
                .any(|r| matches!(r, Redirection::Overwrite(_) | Redirection::Append(_)));

            if i == num - 1 {
                // 最后一个命令：若无重定向，则 stdout 继承
                if stdout_redir {
                    apply_stdout_redirect(&mut command, redirects)?;
                }
                // 否则保持 inherit
            } else {
                // 非最后一个命令：若无重定向，则 stdout 为 piped
                if stdout_redir {
                    apply_stdout_redirect(&mut command, redirects)?;
                } else {
                    command.stdout(Stdio::piped());
                }
            }

            // ----- 处理标准输入 -----
            if i == 0 {
                // 第一个命令：stdin 继承
                command.stdin(Stdio::inherit());
            } else if let Some(prev_stdout) = previous_stdout.take() {
                // 使用前一个命令的 stdout 作为 stdin
                command.stdin(prev_stdout);
            } else {
                // 前一个命令 stdout 被重定向等情况，stdin 继承
                command.stdin(Stdio::inherit());
            }

            // stderr 统一继承
            command.stderr(Stdio::inherit());

            // 生成子进程
            let mut child = command.spawn().map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    ShellError::CommandNotFound(name.clone())
                } else {
                    ShellError::Io(e)
                }
            })?;

            // 如果不是最后一个命令且 stdout 被 piped，取出 pipe 的 stdout 供下一个命令使用
            if i != num - 1 && !stdout_redir {
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
