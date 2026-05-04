use crate::error::ShellError;
use crate::context::ShellContext;

/// 内建命令执行后的控制信号
pub enum ShouldExit {
    Continue,
    Exit,
}

/// 内建命令必须实现的 trait
pub trait Builtin {
    fn execute(&self, args: &[String], context: &mut ShellContext) -> Result<ShouldExit, ShellError>;
}

// 子模块声明
pub mod exit;
pub mod echo;
#[path = "builtins/type.rs"]
pub mod type_cmd;   // 避免与关键字冲突，使用 type_cmd 作为模块名
pub mod pwd;
pub mod cd;

// 重导出常用类型，方便外部 use
pub use exit::ExitBuiltin;
pub use echo::EchoBuiltin;
pub use type_cmd::TypeBuiltin;
pub use pwd::PwdBuiltin;
pub use cd::CdBuiltin;