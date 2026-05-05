use std::collections::HashMap;
use std::fs::File;
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
        let ParsedCommand { name, args, redirection } = cmd;
        let mut redirect_file = None;

        if let Some(redir) = redirection {
            match redir {
                parser::Redirection::Overwrite(filename) => {
                    let f = File::create(filename)?;
                    redirect_file = Some(f);
                }
            }
        }

        let output: &mut dyn Write = match &mut redirect_file {
            Some(f) => f,
            None => &mut io::stdout(),
        };

        // 借用内建命令表（不可变）和上下文（可变）互不冲突
        if let Some(builtin) = self.builtins.get(&name) {
            return builtin.execute(&args, &mut self.context, output);
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

        if let Some(f) = redirect_file.take() {
            command.stdout(f);
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
