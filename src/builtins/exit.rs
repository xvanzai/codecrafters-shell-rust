use super::{Builtin, ShouldExit};
use crate::context::ShellContext;
use crate::error::ShellError;

pub struct ExitBuiltin;

impl Builtin for ExitBuiltin {
    fn name(&self) -> &str {
        "exit"
    }
    fn execute(
        &self,
        _args: &[String],
        _context: &mut ShellContext,
        _writer: &mut dyn std::io::Write,
    ) -> Result<ShouldExit, ShellError> {
        Ok(ShouldExit::Exit)
    }
}
