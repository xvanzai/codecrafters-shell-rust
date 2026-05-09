use std::io::Write;

use crate::{
    builtins::{Builtin, ShouldExit},
    context::ShellContext,
    error::ShellError,
};

pub struct JobsBuiltin;

impl Builtin for JobsBuiltin {
    fn name(&self) -> &str {
        "jobs"
    }

    fn execute(
        &self,
        _args: &[String],
        context: &mut ShellContext,
        writer: &mut dyn Write,
    ) -> Result<ShouldExit, ShellError> {
        for job in context.list_background_jobs() {
            writeln!(writer, "[{}]+  {:<24}{} &", job.id, "Running", job.command)?;
        }
        Ok(ShouldExit::Continue)
    }
}

use std::process::Child;

pub struct Job {
    pub id: usize,
    pub command: String,   // 重建的命令文本
    pub child: Child,
}
