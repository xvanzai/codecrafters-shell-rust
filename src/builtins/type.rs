use super::{Builtin, ShouldExit};
use crate::error::ShellError;
use crate::context::ShellContext;

pub struct TypeBuiltin;

impl Builtin for TypeBuiltin {
    fn execute(&self, args: &[String], context: &mut ShellContext) -> Result<ShouldExit, ShellError> {
        if args.is_empty() {
            return Err(ShellError::BuiltinError("type: missing operand".to_string()));
        }
        for cmd in args {
            if context.builtin_names.contains(cmd) {
                println!("{} is a shell builtin", cmd);
            } else if let Some(path) = context.resolve_cmd(cmd) {
                println!("{} is {}", cmd, path.display());
            } else {
                println!("{}: not found", cmd);
            }
        }
        Ok(ShouldExit::Continue)
    }
}
