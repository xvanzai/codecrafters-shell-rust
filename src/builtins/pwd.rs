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
    ) -> Result<ShouldExit, ShellError> {
        if !args.is_empty() {
            return Err(ShellError::BuiltinError(
                "pwd: too many arguments".to_string(),
            ));
        }
        let current_dir = std::env::current_dir().map_err(|e| {
            ShellError::BuiltinError(format!("pwd: failed to get current directory: {}", e))
        })?;
        println!("{}", current_dir.display());
        Ok(ShouldExit::Continue)
    }
}
