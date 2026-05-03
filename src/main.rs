#[allow(unused_imports)]
use std::io::{self, Write};

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
            [COMMAND_TYPE, arg @ (COMMAND_ECHO | COMMAND_EXIT | COMMAND_TYPE)] => println!("{arg} is a shell builtin"),
            [COMMAND_TYPE, arg @ ..] => println!("{}: not found", arg[0]), // 当使用 type 不添加任何参数时 会匹配到该分支 导致发生panic：index out of bounds: the len is 0 but the index is 0
            _ => println!("{}: command not found", user_command.trim()),
        }
    }
}
