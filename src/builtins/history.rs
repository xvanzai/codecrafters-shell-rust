use crate::builtins::Builtin;

pub struct HistoryBuiltin;

impl Builtin for HistoryBuiltin {
    fn name(&self) -> &str {
        "history"
    }

    fn execute(
        &self,
        args: &[String],
        context: &mut crate::context::ShellContext,
        writer: &mut dyn std::io::prelude::Write,
    ) -> Result<super::ShouldExit, crate::error::ShellError> {
        // -c 清除历史
        if args.first().map(|s| s.as_str()) == Some("-c") {
            context.request_clear_history = true;
            return Ok(super::ShouldExit::Continue);
        }

        // 输出带编号的历史
        for (i, entry) in context.history_entries.iter().enumerate() {
            writeln!(writer, "{:>5}  {}", i + 1, entry)?;
        }

        Ok(super::ShouldExit::Continue)
    }
}
