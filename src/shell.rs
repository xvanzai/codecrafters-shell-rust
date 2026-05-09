use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::process::Command;

use crate::builtins::{self, Builtin, ShouldExit};
use crate::completer::{ShellHelper, create_editor_with_helper};
use crate::context::ShellContext;
use crate::error::ShellError;
use crate::parser::{self, ParsedCommand, Redirection};

pub struct Shell {
    builtins: HashMap<String, Box<dyn Builtin>>,
    context: ShellContext,
    editor: rustyline::Editor<ShellHelper, rustyline::history::DefaultHistory>,
    background_jobs: Vec<std::process::Child>, // 存储后台作业的句柄
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
            background_jobs: Vec::new(),
        }
    }

    pub fn run(&mut self) -> Result<(), ShellError> {
        loop {
            let readline = self.editor.readline("$ ");

            match readline {
                Ok(line) => {
                    // 处理用户输入
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    let cmd = match parser::parse(trimmed) {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("{}", e); // 语法错误直接输出到全局 stderr
                            continue;
                        }
                    };

                    match self.execute_command(cmd) {
                        Ok(ShouldExit::Continue) => {}
                        Ok(ShouldExit::Exit) => break,
                        Err(e) => eprintln!("{}", e), // 其他错误
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
                    println!("[{}] {}", self.background_jobs.len() + 1, child.id());
                    self.background_jobs.push(child);
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
