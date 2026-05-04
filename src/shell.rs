use std::collections::HashMap;
use std::io::{self, Write};
use std::process::Command;

use crate::builtins::{self, Builtin, ShouldExit};
use crate::context::ShellContext;
use crate::error::ShellError;
use crate::parser::{self, ParsedCommand};

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
        // 借用内建命令表（不可变）和上下文（可变）互不冲突
        if let Some(builtin) = self.builtins.get(&cmd.name) {
            return builtin.execute(&cmd.args, &mut self.context);
        }

        // 外部命令
        let path = self
            .context
            .resolve_cmd(&cmd.name)
            .ok_or_else(|| ShellError::CommandNotFound(cmd.name.clone()))?;

        let cmd_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ShellError::CommandNotFound(cmd.name.clone()))?;
        let status = Command::new(cmd_name)
            .args(&cmd.args)
            .status()
            .map_err(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    ShellError::CommandNotFound(cmd.name.clone())
                } else {
                    ShellError::Io(e)
                }
            })?;

        if !status.success() {
            eprintln!("{}: exited with code {}", cmd.name, status);
        }

        Ok(ShouldExit::Continue)
    }
}
