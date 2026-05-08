use crate::{
    builtins::{Builtin, ShouldExit},
    error::ShellError,
};

pub struct CompleteBuiltin;

impl Builtin for CompleteBuiltin {
    fn execute(
        &self,
        args: &[String],
        context: &mut crate::context::ShellContext,
        _writer: &mut dyn std::io::Write,
    ) -> Result<ShouldExit, ShellError> {
        match args {
            [flag, command_name, ..] if flag == "-p" => {
                if let Some(path) = context.get_complete_command_path(command_name) {
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
                    context.register_complete_command(command_name, path);
                }
            }
            [..] => {}
        }
        Ok(ShouldExit::Continue)
    }
}
