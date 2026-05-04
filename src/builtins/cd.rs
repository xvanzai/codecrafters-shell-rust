use std::path::Path;

use crate::{
    builtins::{Builtin, ShouldExit},
    context::ShellContext,
    error::ShellError,
};

pub struct CdBuiltin;

impl Builtin for CdBuiltin {
    fn execute(
        &self,
        args: &[String],
        context: &mut ShellContext,
    ) -> Result<ShouldExit, ShellError> {
        let target = if args.is_empty() {
            context.env_vars.get("HOME")
                .cloned()
                .ok_or_else(|| ShellError::BuiltinError("cd: HOME not set".to_string()))?
        } else {
            args[0].clone()
        };

        let path = Path::new(&target);
        if !path.is_dir() {
            return Err(ShellError::BuiltinError(format!(
                "cd: {}: No such directory",
                target
            )));
        } else {
            std::env::set_current_dir(path).map_err(|e| {
                ShellError::BuiltinError(format!("cd: {}: {}", target, e))
            })?;
        }
       
        Ok(ShouldExit::Continue)
    }
}
