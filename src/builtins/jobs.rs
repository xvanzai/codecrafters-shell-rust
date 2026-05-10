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
        let jobs = context.list_background_jobs_mut();
        let len = jobs.len();
        let mut remove_index = Vec::new();
        for (i, job) in jobs.iter_mut().enumerate() {
            let marker = if i == len - 1 {
                "+"
            } else if i == len - 2 {
                "-"
            } else {
                " "
            };
            match job.child.try_wait() {
                Ok(Some(_status)) => {
                    // 进程已退出 → 显示 Done，无 &，准备移除
                    writeln!(
                        writer,
                        "[{}]{}  {:<24}{}",
                        job.id, marker, "Done", job.command
                    )?;
                    remove_index.push(job.id);
                }
                Ok(None) => {
                    // 仍在运行 → 显示 Running，带 &
                    writeln!(
                        writer,
                        "[{}]{}  {:<24}{} &",
                        job.id, marker, "Running", job.command
                    )?;
                }
                Err(_) => {
                    // 检查出错也视为结束，显示 Done 并移除
                    writeln!(
                        writer,
                        "[{}]{}  {:<24}{}",
                        job.id, marker, "Done", job.command
                    )?;
                    remove_index.push(job.id);
                }
            }
        }

        // 移除已结束的作业
        jobs.retain(|job| !remove_index.contains(&job.id));

        Ok(ShouldExit::Continue)
    }
}

use std::process::Child;

pub struct Job {
    pub id: usize,
    pub command: String, // 重建的命令文本
    pub child: Child,
}
