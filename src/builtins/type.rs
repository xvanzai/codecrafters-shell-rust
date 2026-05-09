use super::{Builtin, ShouldExit};
use crate::context::ShellContext;
use crate::error::ShellError;

pub struct TypeBuiltin;

impl Builtin for TypeBuiltin {
    fn name(&self) -> &str {
        "type"
    }
    fn execute(
        &self,
        args: &[String],
        context: &mut ShellContext,
        writer: &mut dyn std::io::Write,
    ) -> Result<ShouldExit, ShellError> {
        if args.is_empty() {
            return Err(ShellError::BuiltinError(
                "type: missing operand".to_string(),
            ));
        }
        for cmd in args {
            if context.builtin_names.contains(cmd) {
                writeln!(writer, "{} is a shell builtin", cmd).unwrap();
            } else if let Some(path) = context.resolve_cmd(cmd) {
                writeln!(writer, "{} is {}", cmd, path.display()).unwrap();
            } else {
                writeln!(writer, "{}: not found", cmd).unwrap();
            }
        }
        Ok(ShouldExit::Continue)
    }
}
