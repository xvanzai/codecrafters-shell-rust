#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut user_command = String::new();
        io::stdin().read_line(&mut user_command).unwrap();
        println!("{}: command not found", user_command.trim())
    }
}
