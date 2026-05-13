use crate::builtins::Builtin;

pub struct HistoryBuiltin;

impl Builtin for HistoryBuiltin {
    fn name(&self) -> &str {
        "history"
    }

    fn execute(
        &self,
        _args: &[String],
        _context: &mut crate::context::ShellContext,
        _writer: &mut dyn std::io::prelude::Write,
    ) -> Result<super::ShouldExit, crate::error::ShellError> {
        todo!()
    }
}
