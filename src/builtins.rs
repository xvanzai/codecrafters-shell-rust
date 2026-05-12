use std::io::Write;

use crate::context::ShellContext;
use crate::error::ShellError;

/// 内建命令执行后的控制信号
pub enum ShouldExit {
    Continue,
    Exit,
}

/// 内建命令必须实现的 trait
pub trait Builtin {
    fn name(&self) -> &str;
    fn execute(
        &self,
        args: &[String],
        context: &mut ShellContext,
        writer: &mut dyn Write,
    ) -> Result<ShouldExit, ShellError>;
    /// 该内建命令是否需要从标准输入读取数据。
    /// 默认不需要输入，若需要请覆盖返回 true。
    fn needs_stdin(&self) -> bool {
        false
    }
}

// 子模块声明
pub mod cd;
pub mod complete;
pub mod echo;
pub mod exit;
pub mod jobs;
pub mod pwd;
#[path = "builtins/type.rs"]
pub mod type_cmd; // 避免与关键字冲突，使用 type_cmd 作为模块名

// 重导出常用类型，方便外部 use
pub use cd::CdBuiltin;
pub use complete::CompleteBuiltin;
pub use echo::EchoBuiltin;
pub use exit::ExitBuiltin;
pub use jobs::JobsBuiltin;
pub use pwd::PwdBuiltin;
pub use type_cmd::TypeBuiltin;
