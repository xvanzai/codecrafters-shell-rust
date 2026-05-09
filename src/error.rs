use std::fmt;
use std::io;

#[derive(Debug)]
pub enum ShellError {
    Io(io::Error),
    ParseError(String),
    CommandNotFound(String),
    BuiltinError(String),
}

impl From<io::Error> for ShellError {
    fn from(err: io::Error) -> Self {
        ShellError::Io(err)
    }
}

impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShellError::Io(e) => write!(f, "I/O error: {}", e),
            ShellError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ShellError::CommandNotFound(cmd) => write!(f, "{}: command not found", cmd),
            ShellError::BuiltinError(msg) => write!(f, "{}", msg),
        }
    }
}
