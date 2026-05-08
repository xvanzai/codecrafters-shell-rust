// src/completer.rs
use crate::context::ShellContext;
use rustyline::completion::{Completer as CompleterTarit, FilenameCompleter, Pair};
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use rustyline::{Completer, Context, Helper, Highlighter, Hinter, Validator};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::vec;

pub struct ShellCompleter {
    commands: HashSet<String>,
    complete_command: Rc<RefCell<HashMap<String, String>>>,
    filename_completer: FilenameCompleter,
}

impl ShellCompleter {
    /// 根据 ShellContext 构建命令集合（内建 + PATH 外部命令）
    pub fn new(context: &ShellContext) -> Self {
        let mut commands = HashSet::new();

        // 内建命令
        for name in &context.builtin_names {
            commands.insert(name.clone());
        }

        // path 外部命令
        if let Some(path) = context.env_vars.get("PATH") {
            for dir in path.split(':') {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            commands.insert(name.to_string());
                        }
                    }
                }
            }
        }

        ShellCompleter {
            commands,
            complete_command: Rc::clone(&context.complete_command),
            filename_completer: FilenameCompleter::new(),
        }
    }
}

impl CompleterTarit for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        // 1. 找到光标所在单词的起始位置
        let word_start = line[..pos]
            .rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        let word = &line[word_start..pos];

        // 2. 根据上下文分发
        let (upos, mut candidates) = if word_start == 0 && !word.contains('/') {
            // rustyline 的 completer 是基于光标位置的，所以我们需要判断当前光标所在单词是否是第一个单词（命令）。
            // 如果是第一个单词且不包含路径分隔符（/），则进行命令补全；
            let ca: Vec<_> = self
                .commands
                .iter()
                .filter(|cmd| cmd.starts_with(word))
                .map(|cmd| Pair {
                    display: cmd.clone(),
                    replacement: format!("{} ", cmd.clone()),
                })
                .collect();
            (word_start, ca)
        } else {
            let first_word = line.split_whitespace().next().unwrap_or("");
            if let Some(script_path) = self.complete_command.borrow().get(first_word) {
                // 如果第一个单词有对应的补全规范，则执行补全脚本获取候选项
                let c = run_completer_script(script_path, line, pos);
                (
                    word_start,
                    c.into_iter()
                        .map(|pair| Pair {
                            display: pair.display.clone(),
                            replacement: format!("{} ", pair.replacement),
                        })
                        .collect(),
                )
            } else {
                // 否则进行文件补全
                let (u, c) = self.filename_completer.complete(line, pos, _ctx)?;
                (
                    u,
                    c.into_iter()
                        .map(|pair| Pair {
                            display: if pair.replacement.ends_with('/') {
                                pair.display.clone() + "/"
                            } else {
                                pair.display.clone()
                            },
                            replacement: if !pair.replacement.ends_with('/')
                                && !pair.replacement.ends_with(' ')
                            {
                                format!("{} ", pair.replacement)
                            } else {
                                pair.replacement
                            },
                        })
                        .collect(),
                )
            }
        };

        candidates.sort_by(|a, b| a.display.cmp(&b.display));

        Ok((upos, candidates))
    }
}

#[derive(Completer, Validator, Highlighter, Hinter)]
pub struct ShellHelper {
    #[rustyline(Completer)]
    completer: ShellCompleter,
}

impl Helper for ShellHelper {}

fn create_shell_helper(context: &ShellContext) -> ShellHelper {
    let completer = ShellCompleter::new(context);
    ShellHelper { completer }
}

pub fn create_editor_with_helper(
    context: &ShellContext,
) -> rustyline::Editor<ShellHelper, rustyline::history::DefaultHistory> {
    let mut editor = rustyline::Editor::new().expect("Failed to create rustyline editor");
    let helper = create_shell_helper(context);
    editor.set_helper(Some(helper));
    editor.set_completion_type(rustyline::CompletionType::List);
    editor
}

fn run_completer_script(script_path: &str, line: &str, pos: usize) -> Vec<Pair> {
    use std::process::Command;
    let text_before_cursor = &line[..pos];

    // 命令名：第一个 token（如果存在）
    let tokens: Vec<&str> = text_before_cursor.split_whitespace().collect();
    if tokens.is_empty() {
        return vec![];
    }

    let command_name = tokens[0].to_string();

    // 确定当前要补全的单词 (argv[2]) 和前一个单词 (argv[3])
    let (current_word, previous_word) =
        if pos > 0 && text_before_cursor.as_bytes().last() == Some(&b' ') {
            // 光标紧跟在空格后：当前单词为空，前一个单词为最后一个完整的单词
            let prev = tokens.last().map(|s| s.to_string()).unwrap_or_default();
            ("".to_string(), prev)
        } else {
            // 光标在单词内部
            let current = tokens.last().map(|s| s.to_string()).unwrap_or_default();
            let prev = if tokens.len() >= 2 {
                tokens[tokens.len() - 2].to_string()
            } else {
                "".to_string()
            };
            (current, prev)
        };
    let output = match Command::new(script_path)
        .args(&[command_name, current_word, previous_word])
        .env("COMP_LINE", line)
        .env("COMP_POINT", pos.to_string())
        .output()
    {
        Ok(o) => o,
        Err(_) => return vec![],
    };

    if !output.status.success() {
        return vec![];
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| Pair {
            display: l.to_string(),
            replacement: format!("{} ", l),
        })
        .collect()
}
