use super::{Builtin, ShouldExit};
use crate::error::ShellError;
use crate::context::ShellContext;

pub struct EchoBuiltin;

impl Builtin for EchoBuiltin {
    fn execute(&self, args: &[String], _context: &mut ShellContext) -> Result<ShouldExit, ShellError> {
        println!("{}", args.join(" "));
        Ok(ShouldExit::Continue)
    }
}