mod builtins;
mod completer;
mod context;
mod error;
mod parser;
mod resolver;
mod shell;

use error::ShellError;
use shell::Shell;

fn main() -> Result<(), ShellError> {
    let mut shell = Shell::new();
    if let Err(e) = shell.run() {
        eprintln!("Shell exited with error: {}", e);
        std::process::exit(1);
    }
    Ok(())
}
