use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const COMMAND_EXIT: &str = "exit";
const COMMAND_TYPE: &str = "type";
const COMMAND_ECHO: &str = "echo";

const BUILTINS: [&str; 3] = [COMMAND_EXIT, COMMAND_TYPE, COMMAND_ECHO];

fn is_builtin(cmd: &str) -> bool {
    BUILTINS.contains(&cmd)
}

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut user_command = String::new();
        // EOF 时退出循环
        if io::stdin().read_line(&mut user_command).is_err() {
            break;
        }

        let parts: Vec<&str> = user_command.split_whitespace().collect();
        match parts.as_slice() {
            [] => continue, // 空行忽略
            [COMMAND_EXIT] => break,
            [COMMAND_ECHO, args @ ..] => println!("{}", args.join(" ")),
            [COMMAND_TYPE] => println!("type: missing operand"),
            [COMMAND_TYPE, args @ ..] => {
                for &arg in args {
                    if is_builtin(arg) {
                        println!("{} is a shell builtin", arg);
                    } else if let Some(path) = find_cmd_in_path(arg) {
                        println!("{} is {}", arg, path.to_string_lossy());
                    } else {
                        println!("{}: not found", arg);
                    }
                }
            }
            [cmd, args @ ..] => {
                // 处理外部命令
                if let Some(path) = resolve_command_path(cmd) {
                    let status = Command::new(&path)
                        .args(args)
                        .status()
                        .expect("Failed to execute command");
                    if !status.success() {
                        // 命令执行失败（非零退出码）
                        std::process::exit(status.code().unwrap_or(1));
                    }
                } else {
                    println!("{}: command not found", cmd);
                }
            }
        }
    }
}

fn find_cmd_in_path(cmd: &str) -> Option<PathBuf> {
    let path = std::env::var("PATH").ok()?;
    path.split(':')
        .map(Path::new)
        .filter(|dir| dir.is_dir())
        .filter_map(|dir| fs::read_dir(dir).ok())
        .flat_map(|entries| entries.filter_map(Result::ok))
        .find(|entry| {
            entry.file_name() == cmd
                && entry
                    .metadata()
                    .ok()
                    .map(|meta| meta.is_file() && (meta.permissions().mode() & 0o111 != 0))
                    .unwrap_or(false)
        })
        .map(|entry| entry.path())
}

fn resolve_command_path(cmd: &str) -> Option<PathBuf> {
    if cmd.contains('/') {
        let path = Path::new(cmd);
        if path.is_file() && is_executable(path) {
            Some(path.to_path_buf())
        } else {
            None
        }
    } else {
        find_cmd_in_path(cmd)
    }
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .ok()
        .map(|meta| meta.is_file() && (meta.permissions().mode() & 0o111 != 0))
        .unwrap_or(false)
}
