use crate::error::ShellError;

#[derive(Debug, Clone)]
pub enum Redirection {
    /// 输出重定向到文件（截断）
    Overwrite(String),
    /// 输出重定向到文件（追加）
    Append(String),
    /// 标准错误重定向到文件（截断）
    StderrOverwrite(String),
    /// 标准错误重定向到文件（追加）
    StderrAppend(String),
}

/// 解析后的命令
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub name: String,
    pub args: Vec<String>,
    pub redirects: Vec<Redirection>,
    pub is_background: bool,
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub commands: Vec<ParsedCommand>,
}

pub fn parse(input: &str) -> Result<ParsedCommand, ShellError> {
    let mut tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err(ShellError::ParseError("empty command".to_string()));
    }

    // 检查是否以 & 结尾，表示后台执行
    let is_background = if tokens.last().map(|s| s.as_str()) == Some("&") {
        tokens.pop();
        true
    } else {
        false
    };

    // 提取重定向，并移除相关 token
    let redirections = extract_redirects(&mut tokens)?;

    if tokens.is_empty() {
        return Err(ShellError::ParseError("missing command".to_string()));
    }

    Ok(ParsedCommand {
        name: tokens.remove(0),
        args: tokens,
        redirects: redirections,
        is_background,
    })
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

/// 提取所有重定向，从 tokens 中移除相关 token 对，返回重定向列表（保持出现顺序）。
fn extract_redirects(tokens: &mut Vec<String>) -> Result<Vec<Redirection>, ShellError> {
    let mut redirects = Vec::new();
    let mut i = 0;
    // 循环中 tokens 长度可能变化，使用 while 并检查边界
    while i < tokens.len() {
        let token = &tokens[i];
        let redir_type: Option<fn(String) -> Redirection> = match token.as_str() {
            ">" | "1>" => Some(Redirection::Overwrite),
            "2>" => Some(Redirection::StderrOverwrite),
            ">>" | "1>>" => Some(Redirection::Append),
            "2>>" => Some(Redirection::StderrAppend),
            _ => None,
        };

        if let Some(constructor) = redir_type {
            // 操作符后必须有文件名
            let filename = next_token_as_filename(tokens, i)?;
            redirects.push(constructor(filename));
            // 移除操作符和文件名
            tokens.drain(i..=i + 1);
            // i 不变，因为当前位置已经被下一个 token 占据（若存在）
        } else {
            i += 1;
        }
    }
    Ok(redirects)
}

/// 提取当前操作符后的文件名 token。
/// 若不存在则返回错误。
fn next_token_as_filename(tokens: &[String], op_index: usize) -> Result<String, ShellError> {
    if op_index + 1 >= tokens.len() {
        return Err(ShellError::ParseError(format!(
            "missing filename for '{}'",
            tokens[op_index]
        )));
    }
    Ok(tokens[op_index + 1].clone())
}

/// 解析含有管道的命令行，返回一个 Pipeline
pub fn parse_pipeline(input: &str) -> Result<Pipeline, ShellError> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err(ShellError::ParseError("empty command".to_string()));
    }

    let mut commands = Vec::new();
    let mut cur_tokens = Vec::new();

    for token in tokens {
        if token == "|" {
            if cur_tokens.is_empty() {
                return Err(ShellError::ParseError("unexpected |".into()));
            }
            let cmd = build_command_from_tokens(&cur_tokens, false)?; // false 表示不支持 &
            commands.push(cmd);
            cur_tokens.clear();
        } else {
            cur_tokens.push(token);
        }
    }

    if cur_tokens.is_empty() {
        return Err(ShellError::ParseError("unexpected end of pipeline".into()));
    }
    let cmd = build_command_from_tokens(&cur_tokens, false)?;
    commands.push(cmd);

    Ok(Pipeline { commands })
}

/// 从 tokens 构建 ParsedCommand，选择是否允许后台 & 标记
fn build_command_from_tokens(
    tokens: &[String],
    allow_background: bool,
) -> Result<ParsedCommand, ShellError> {
    let mut words = tokens.to_vec();

    // 检查后台符号（仅当允许时）
    let is_background = if allow_background {
        if words.last().map(|s| s.as_str()) == Some("&") {
            words.pop();
            true
        } else {
            false
        }
    } else {
        false
    };

    let redirects = extract_redirects(&mut words)?; // 复用已有的重定向提取

    if words.is_empty() {
        return Err(ShellError::ParseError("missing command".into()));
    }

    let name = words.remove(0);
    Ok(ParsedCommand {
        name,
        args: words,
        redirects,
        is_background,
    })
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
