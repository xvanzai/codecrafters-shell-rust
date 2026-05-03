use crate::error::ShellError;

/// 解析后的命令
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub name: String,
    pub args: Vec<String>,
}

pub fn parse(input: &str) -> Result<ParsedCommand, ShellError> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err(ShellError::ParseError("empty command".to_string()));
    }
    let name = tokens[0].clone();
    let args = tokens[1..].to_vec();
    Ok(ParsedCommand { name, args })
}

fn tokenize(input: &str) -> Result<Vec<String>, ShellError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false; // 当前是否处于单引号内部
    let mut in_double = false; // 当前是否处于双引号内部

    while let Some(&c) = chars.peek() {
        if in_single {
            chars.next();
            if c == '\'' {
                in_single = false;
            } else {
                current.push(c);
            }
            continue;
        }

        if in_double {
            chars.next();
            match c {
                '"' => in_double = false,
                '\\' => {
                    if let Some(&next) = chars.peek() {
                        chars.next();
                        current.push(next);
                    } else {
                        return Err(ShellError::ParseError(
                            "trailing backslash in double quotes".into(),
                        ));
                    }
                }
                _ => current.push(c),
            }
            continue;
        }

        match c {
            ' ' | '\t' => {
                chars.next();
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            '\'' => {
                chars.next();
                in_single = true;
            }
            '"' => {
                chars.next();
                in_double = true;
            }
            '\\' => {
                chars.next();
                if let Some(&next) = chars.peek() {
                    chars.next();
                    current.push(next);
                } else {
                    return Err(ShellError::ParseError("trailing backslash".into()));
                }
            }
            _ => {
                chars.next();
                current.push(c);
            }
        }
    }

    if in_single {
        return Err(ShellError::ParseError("unclosed single quote".into()));
    }
    if in_double {
        return Err(ShellError::ParseError("unclosed double quote".into()));
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        let cmd = parse("echo hello").unwrap();
        assert_eq!(cmd.name, "echo");
        assert_eq!(cmd.args, vec!["hello"]);
    }

    #[test]
    fn test_quote() {
        let cmd = parse(r#"echo "a b" c"#).unwrap();
        assert_eq!(cmd.args, vec!["a b", "c"]);
    }

    #[test]
    fn test_unclosed_quote() {
        assert!(parse("echo \"hello").is_err());
    }
}