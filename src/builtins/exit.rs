use super::{Builtin, ShouldExit};
use crate::error::ShellError;
use crate::context::ShellContext;

pub struct ExitBuiltin;

impl Builtin for ExitBuiltin {
    fn execute(&self, _args: &[String], _context: &mut ShellContext, _writer: &mut dyn std::io::Write) -> Result<ShouldExit, ShellError> {
        Ok(ShouldExit::Exit)
    }
}