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
        writer: &mut dyn std::io::Write,
    ) -> Result<super::ShouldExit, crate::error::ShellError> {
        match args.first().map(|s| s.as_str()) {
            Some("-c") => {
                // 删除历史
                context.request_clear_history = true;
                return Ok(super::ShouldExit::Continue);
            }
            Some("-r") => {
                // 从指定文件读取历史（先清空历史）
                if let Some(file_path) = args.get(1) {
                    context.request_load_history = Some(file_path.clone());
                } else {
                    writeln!(writer, "history: -r requires a file path argument")?;
                }
                return Ok(super::ShouldExit::Continue);
            }
            Some("-w") => {
                // 将历史写入到指定文件
                if let Some(file_path) = args.get(1) {
                    context.request_write_history = Some(file_path.clone());
                } else {
                    writeln!(writer, "history: -w requires a file path argument")?;
                }
                return Ok(super::ShouldExit::Continue);
            }
            _ => {} // 其他情况包括数字或空
        }

        let entries = &context.history_entries;
        let total = entries.len();
        let to_show = match args.first().and_then(|s| s.parse::<usize>().ok()) {
            Some(0) => &[][..],
            Some(n) => {
                let start = total.saturating_sub(n); // 若 n > total，则 start = 0
                &entries[start..]
            }
            None => &entries[..],
        };

        // 输出带编号的历史
        let first_index = total - to_show.len() + 1; // 第一条的行号
        for (i, entry) in to_show.iter().enumerate() {
            writeln!(writer, "{:>5}  {}", first_index + i, entry)?;
        }

        Ok(super::ShouldExit::Continue)
    }
}
