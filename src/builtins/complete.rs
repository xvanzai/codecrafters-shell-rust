use std::{cell::RefCell, collections::HashMap};

use crate::{
    builtins::{Builtin, ShouldExit},
    error::ShellError,
};

pub struct CompleteBuiltin {
    complete_command: RefCell<HashMap<String, String>>, // 存储命令与其补全规范的映射
}

impl CompleteBuiltin {
    pub fn new() -> Self {
        Self {
            complete_command: RefCell::new(HashMap::new()),
        }
    }
}

impl Builtin for CompleteBuiltin {
    fn execute(
        &self,
        args: &[String],
        _context: &mut crate::context::ShellContext,
        _writer: &mut dyn std::io::Write,
    ) -> Result<ShouldExit, ShellError> {
        match args {
            [flag, command_name, ..] if flag == "-p" => {
                if let Some(path) = self.complete_command.borrow().get(command_name) {
                    writeln!(_writer, "complete -C '{}' {}", path, command_name)?;
                } else {
                    return Err(ShellError::BuiltinError(format!(
                        "complete: {}: no completion specification",
                        command_name
                    )));
                }
            }
            [flag, path, command_name, ..] if flag == "-C" => {
                if !path.is_empty() && !command_name.is_empty() {
                    self.complete_command.borrow_mut().insert(command_name.clone(), path.clone());
                }
            }
            [..] => {}
        }
        Ok(ShouldExit::Continue)
    }
}
