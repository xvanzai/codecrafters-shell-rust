use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const COMMAND_EXIT: &str = "exit";
const COMMAND_TYPE: &str = "type";
const COMMAND_ECHO: &str = "echo";

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut user_command = String::new();
        io::stdin().read_line(&mut user_command).unwrap();

        match user_command
            .split_whitespace()
            .collect::<Vec<&str>>()
            .as_slice()
        {
            [COMMAND_EXIT] => break,
            [COMMAND_ECHO, arg @ ..] => println!("{}", arg.join(" ")),
            [
                COMMAND_TYPE,
                arg @ (COMMAND_ECHO | COMMAND_EXIT | COMMAND_TYPE),
            ] => println!("{arg} is a shell builtin"),
            [COMMAND_TYPE] => println!("type: missing operand"),
            [COMMAND_TYPE, arg @ ..] => println!("{}: not found", arg[0]),
            [cmd, ..] if let Some(path) = find_cmd_in_path(cmd) => {
                println!("{} is {}", cmd, path.to_string_lossy())
            }
            _ => println!("{}: command not found", user_command.trim()),
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
