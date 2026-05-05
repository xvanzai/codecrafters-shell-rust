use super::{Builtin, ShouldExit};
use crate::error::ShellError;
use crate::context::ShellContext;

pub struct EchoBuiltin;

impl Builtin for EchoBuiltin {
    fn execute(&self, args: &[String], _context: &mut ShellContext, writer: &mut dyn std::io::Write) -> Result<ShouldExit, ShellError> {
        writeln!(writer, "{}", args.join(" ")).unwrap();
        Ok(ShouldExit::Continue)
    }
}