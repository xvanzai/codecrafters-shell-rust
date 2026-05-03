use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use crate::resolver::resolve_path;

pub struct ShellContext {
    pub env_vars: HashMap<String, String>,
    cmd_cache: HashMap<String, PathBuf>,
    /// 供 type 等命令判断是否内建
    pub builtin_names: Vec<String>,
}

impl ShellContext {
    pub fn new() -> Self {
        ShellContext {
            env_vars: env::vars().collect(),
            cmd_cache: HashMap::new(),
            builtin_names: Vec::new(),
        }
    }

    /// 查找命令（优先使用缓存），返回完整路径
    pub fn resolve_cmd(&mut self, cmd: &str) -> Option<PathBuf> {
        if let Some(path) = self.cmd_cache.get(cmd) {
            return Some(path.clone());
        }
        let path_env = self.env_vars.get("PATH").map(String::as_str);
        if let Some(resolved) = resolve_path(cmd, path_env) {
            self.cmd_cache.insert(cmd.to_string(), resolved.clone());
            Some(resolved)
        } else {
            None
        }
    }

    /// 注册内建命令名（在 Shell 初始化时调用）
    pub fn register_builtin_name(&mut self, name: &str) {
        self.builtin_names.push(name.to_string());
    }
}