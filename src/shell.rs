use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::process::Command;

use crate::builtins::{self, Builtin, ShouldExit};
use crate::context::ShellContext;
use crate::error::ShellError;
use crate::parser::{self, ParsedCommand, Redirection};

pub struct Shell {
    builtins: HashMap<String, Box<dyn Builtin>>,
    context: ShellContext,
}

impl Shell {
    pub fn new() -> Self {
        let mut context = ShellContext::new();
        let mut builtins: HashMap<String, Box<dyn Builtin>> = HashMap::new();

        // 注册所有内建命令，并同步内建名称到 context
        let cmd_list: Vec<(&str, Box<dyn Builtin>)> = vec![
            ("exit", Box::new(builtins::ExitBuiltin)),
            ("echo", Box::new(builtins::EchoBuiltin)),
            ("type", Box::new(builtins::TypeBuiltin)),
            ("pwd", Box::new(builtins::PwdBuiltin)),
            ("cd", Box::new(builtins::CdBuiltin)),
        ];

        for (name, builtin) in cmd_list {
            context.register_builtin_name(name);
            builtins.insert(name.to_string(), builtin);
        }

        Shell { builtins, context }
    }

    pub fn run(&mut self) -> Result<(), ShellError> {
        loop {
            print!("$ ");
            io::stdout().flush()?;

            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => break, // EOF
                Ok(_) => {}
                Err(e) => return Err(ShellError::Io(e)),
            }

            let trimmed = input.trim();
            if trimmed.is_empty() {
                continue;
            }

            let cmd = match parser::parse(trimmed) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{}", e);
                    continue;
                }
            };

            let result = self.execute_command(cmd);
            match result {
                Ok(ShouldExit::Continue) => {}
                Ok(ShouldExit::Exit) => break,
                Err(e) => eprintln!("{}", e),
            }
        }
        Ok(())
    }

    fn execute_command(&mut self, cmd: ParsedCommand) -> Result<ShouldExit, ShellError> {
        let ParsedCommand {
            name,
            args,
            redirects,
        } = cmd;

        // 1. 按流分类重定向
        let stdout_redirs: Vec<_> = redirects
            .iter()
            .filter(|r| matches!(r, Redirection::Overwrite(_) | Redirection::Append(_)))
            .collect();
        let stderr_redirs: Vec<_> = redirects
            .iter()
            .filter(|r| matches!(r, Redirection::StderrOverwrite(_) | Redirection::StderrAppend(_)))
            .collect();

        // 2. 对每个流，遍历重定向产生副作用，只保留最后一个文件句柄
        let mut final_stdout = open_redirect_chain(&stdout_redirs)?;
        let mut final_stderr: Option<File> = open_redirect_chain(&stderr_redirs)?;

        // 3. 构造动态输出 target（避免 unwrap）
        let output: &mut dyn Write = match &mut final_stdout {
            Some(f) => f,
            None => &mut io::stdout(),
        };
        let error_output: &mut dyn Write = match &mut final_stderr {
            Some(f) => f,
            None => &mut io::stderr(),
        };

        // 借用内建命令表（不可变）和上下文（可变）互不冲突
        if let Some(builtin) = self.builtins.get(&name) {
            match builtin.execute(&args, &mut self.context, output) {
                Ok(should_exit) => return Ok(should_exit),
                Err(e) => {
                    // 将错误消息写入 builtin 自己的 stderr 流
                    writeln!(error_output, "{}", e).unwrap();
                    return Ok(ShouldExit::Continue);
                }
            }
        }

        // 外部命令
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

        let _status = command.status().map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                ShellError::CommandNotFound(name.clone())
            } else {
                ShellError::Io(e)
            }
        })?;

        // if !status.success() {
        //     eprintln!("{}: exited with code {}", name, status);
        // }

        Ok(ShouldExit::Continue)
    }
}

/// 遍历给定流的重定向列表，依次打开/创建文件（副作用），
/// 仅返回最后一个打开的文件句柄。
fn open_redirect_chain(redirs: &[&Redirection]) -> Result<Option<File>, ShellError> {
    let mut final_file = None;
    for redir in redirs {
        let file = open_redirect_file(redir)?;
        final_file = Some(file); // 旧的自动 drop
    }
    Ok(final_file)
}

/// 根据重定向变体打开文件
fn open_redirect_file(redir: &Redirection) -> Result<File, ShellError> {
    match redir {
        Redirection::Overwrite(filename) | Redirection::StderrOverwrite(filename) => {
            File::create(filename).map_err(|e| {
                ShellError::BuiltinError(format!("failed to open {}: {}", filename, e))
            })
        }
        Redirection::Append(filename) | Redirection::StderrAppend(filename) => OpenOptions::new()
            .create(true)
            .append(true)
            .open(filename)
            .map_err(|e| ShellError::BuiltinError(format!("failed to open {}: {}", filename, e))),
    }
}
