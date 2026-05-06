// src/completer.rs
use crate::context::ShellContext;
use rustyline::completion::{Completer as CompleterTarit, Pair};
use rustyline::error::ReadlineError;
use rustyline::{Completer, Context, Helper, Highlighter, Hinter, Validator};
use std::collections::HashSet;

pub struct ShellCompleter {
    commands: HashSet<String>,
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

        ShellCompleter { commands }
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
        // 仅补全第一个单词（命令名），且光标必须位于行尾或单词末尾
        if let Some(first_word_end) = line.find(|c: char| c.is_whitespace())
            && pos > first_word_end
        {
            return Ok((0, Vec::new()));
        }

        // 截取光标前的单词作为匹配前缀
        let prefix = &line[..pos];
        let candidates: Vec<Pair> = self
            .commands
            .iter()
            .filter(|cmd| cmd.starts_with(prefix))
            .map(|cmd| Pair {
                display: cmd.clone(),
                replacement: format!("{} ", cmd.clone()),
            })
            .collect();

        // start 是替换的起始位置，这里是 0（替换整个单词）
        Ok((0, candidates))
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

pub fn create_editor_with_helper(context: &ShellContext) -> rustyline::Editor<ShellHelper, rustyline::history::DefaultHistory> {
    let mut editor = rustyline::Editor::new().expect("Failed to create rustyline editor");
    let helper = create_shell_helper(context);
    editor.set_helper(Some(helper));
    editor
}
