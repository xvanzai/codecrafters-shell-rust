use crate::{
    builtins::{Builtin, ShouldExit},
    error::ShellError,
};

pub struct CompleteBuiltin;

impl Builtin for CompleteBuiltin {
    fn execute(
        &self,
        args: &[String],
        _context: &mut crate::context::ShellContext,
        _writer: &mut dyn std::io::Write,
    ) -> Result<ShouldExit, ShellError> {
        match args {
            [flag, command_name, ..] if flag == "-p" => {
                // TODO: 实现 -p 选项，输出命令的完整路径
                // 总是返回“未找到规范”的错误，因为我们尚不支持注册
                return Err(ShellError::BuiltinError(format!(
                    "complete: {}: no completion specification",
                    command_name
                )));
            }
            [..] => {}
        }
        Ok(ShouldExit::Continue)
    }
}
