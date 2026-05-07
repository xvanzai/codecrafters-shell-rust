use crate::{
    builtins::{Builtin, ShouldExit},
    error::ShellError,
};

pub struct CompleteBuiltin;

impl Builtin for CompleteBuiltin {
    fn execute(
        &self,
        _args: &[String],
        _context: &mut crate::context::ShellContext,
        _writer: &mut dyn std::io::Write,
    ) -> Result<ShouldExit, ShellError> {
        // TODO: 实现补全逻辑
        Ok(ShouldExit::Continue)
    }
}
