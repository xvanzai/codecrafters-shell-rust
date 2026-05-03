#[allow(unused_imports)]
use std::io::{self, Write};

const COMMAND_EXIT: &str = "exit";

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut user_command = String::new();
        io::stdin().read_line(&mut user_command).unwrap();

        match user_command.trim() {
            COMMAND_EXIT => break,
            cmd if cmd.starts_with("echo") => println!("{}", &cmd[5..]),
            _ => println!("{}: command not found", user_command.trim()),
        }
    }
}
