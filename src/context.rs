use crate::builtins::jobs::Job;
use crate::resolver::resolve_path;
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;

pub struct ShellContext {
    pub env_vars: HashMap<String, String>,
    cmd_cache: HashMap<String, PathBuf>,
    /// 供 type 等命令判断是否内建
    pub builtin_names: Vec<String>,
    pub complete_command: Rc<RefCell<HashMap<String, String>>>,
    pub background_jobs: Vec<Job>, // 存储后台作业的句柄
}

impl ShellContext {
    pub fn new() -> Self {
        ShellContext {
            env_vars: env::vars().collect(),
            cmd_cache: HashMap::new(),
            builtin_names: Vec::new(),
            complete_command: Rc::new(RefCell::new(HashMap::new())),
            background_jobs: Vec::new(),
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

    /// 注册补全规范（在 complete 内建命令执行时调用）
    pub fn register_complete_command(&mut self, command: &str, path: &str) {
        self.complete_command
            .borrow_mut()
            .insert(command.to_string(), path.to_string());
    }

    /// 获取补全规范路径
    pub fn get_complete_command_path(&self, command: &str) -> Option<String> {
        self.complete_command.borrow().get(command).cloned()
    }

    /// 移除补全规范
    pub fn remove_complete_command(&mut self, command: &str) {
        self.complete_command.borrow_mut().remove(command);
    }

    /// 添加后台作业
    pub fn add_background_job(&mut self, child: Job) {
        self.background_jobs.push(child);
    }

    /// 列出后台作业
    pub fn list_background_jobs(&self) -> &Vec<Job> {
        &self.background_jobs
    }
}
