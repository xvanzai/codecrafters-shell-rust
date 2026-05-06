mod shell;
mod context;
mod error;
mod parser;
mod resolver;
mod builtins;
mod completer;

use shell::Shell;
use error::ShellError;

fn main() -> Result<(), ShellError> {
    let mut shell = Shell::new();
    if let Err(e) = shell.run() {
        eprintln!("Shell exited with error: {}", e);
        std::process::exit(1);
    }
    Ok(())
}