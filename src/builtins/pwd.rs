use crate::{
    builtins::{Builtin, ShouldExit},
    context::ShellContext,
    error::ShellError,
};

pub struct PwdBuiltin;

impl Builtin for PwdBuiltin {
    fn execute(
        &self,
        args: &[String],
        _context: &mut ShellContext,
        writer: &mut dyn std::io::Write,
    ) -> Result<ShouldExit, ShellError> {
        if !args.is_empty() {
            return Err(ShellError::BuiltinError(
                "pwd: too many arguments".to_string(),
            ));
        }
        let current_dir = std::env::current_dir().map_err(|e| {
            ShellError::BuiltinError(format!("pwd: failed to get current directory: {}", e))
        })?;
        writeln!(writer, "{}", current_dir.display()).unwrap();
        Ok(ShouldExit::Continue)
    }
}
