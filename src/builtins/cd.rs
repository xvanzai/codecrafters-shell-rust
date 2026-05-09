use std::path::Path;

use crate::{
    builtins::{Builtin, ShouldExit},
    context::ShellContext,
    error::ShellError,
};

pub struct CdBuiltin;

impl Builtin for CdBuiltin {
    fn name(&self) -> &str {
        "cd"
    }

    fn execute(
        &self,
        args: &[String],
        context: &mut ShellContext,
        _writer: &mut dyn std::io::Write,
    ) -> Result<ShouldExit, ShellError> {
        // 1. 获取 HOME，若未设置则回退至根目录
        let home = context
            .env_vars
            .get("HOME")
            .cloned()
            .unwrap_or_else(|| "/".to_string());

        // 2. 构造目标路径（处理无参数与 ~ 展开）
        let target = if args.is_empty() {
            home
        } else if let Some(rest) = args[0].strip_prefix('~') {
            // 去掉 ~，并移除紧随的 '/'，转为相对路径进行拼接
            let relative = rest.trim_start_matches('/');
            Path::new(&home)
                .join(relative)
                .to_str()
                .ok_or_else(|| {
                    ShellError::BuiltinError(format!("cd: bad path after ~: {}", args[0]))
                })?
                .to_string()
        } else {
            args[0].clone()
        };

        // 3. 验证路径为目录
        let path = Path::new(&target);
        if !path.is_dir() {
            return Err(ShellError::BuiltinError(format!(
                "cd: {}: No such file or directory",
                target
            )));
        }

        // 4. 切换目录
        std::env::set_current_dir(path)
            .map_err(|e| ShellError::BuiltinError(format!("cd: {}: {}", target, e)))?;

        Ok(ShouldExit::Continue)
    }
}
