use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// 根据 PATH 查找可执行文件，若 cmd 包含 '/' 则视为直接路径
pub fn resolve_path(cmd: &str, path_env: Option<&str>) -> Option<PathBuf> {
    if cmd.contains('/') {
        let path = Path::new(cmd);
        if path.is_file() && is_executable(path) {
            return Some(path.to_path_buf());
        }
        return None;
    }

    let path_str = path_env.unwrap_or("/usr/bin:/bin");
    for dir in path_str.split(':') {
        let candidate = Path::new(dir).join(cmd);
        if candidate.is_file() && is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|meta| meta.is_file() && (meta.permissions().mode() & 0o111 != 0))
        .unwrap_or(false)
}
