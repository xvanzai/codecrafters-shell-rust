use super::{Builtin, ShouldExit};
use crate::context::ShellContext;
use crate::error::ShellError;

pub struct EchoBuiltin;

impl Builtin for EchoBuiltin {
    fn name(&self) -> &str {
        "echo"
    }
    fn execute(
        &self,
        args: &[String],
        _context: &mut ShellContext,
        writer: &mut dyn std::io::Write,
    ) -> Result<ShouldExit, ShellError> {
        writeln!(writer, "{}", args.join(" ")).unwrap();
        Ok(ShouldExit::Continue)
    }
}
