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

        let entries = &context.history_entries;
        let total = entries.len();
        let to_show = match args.first().and_then(|s| s.parse::<usize>().ok()) {
            Some(0) => &[][..],
            Some(n) => {
                let start = total.saturating_sub(n); // 若 n > total，则 start = 0
                &entries[start..]
            }
            None => &entries[..], // 无参数或参数不是数字，显示全部
        };

        // 输出带编号的历史
        let first_index = total - to_show.len() + 1; // 第一条的行号
        for (i, entry) in to_show.iter().enumerate() {
            writeln!(writer, "{:>5}  {}", first_index + i, entry)?;
        }

        Ok(super::ShouldExit::Continue)
    }
}
