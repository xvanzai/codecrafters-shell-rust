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
        _context: &mut ShellContext,
        _writer: &mut dyn Write,
    ) -> Result<ShouldExit, ShellError> {
        // TODO: 实现 jobs 内建命令，列出当前 shell 中的后台作业
        Ok(ShouldExit::Continue)
    }
}
