// src/cli/prompt.rs — 终端交互组件（纯文本，无 ratatui）

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::io::{stdout, Write};

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub key: &'static str,
    pub label: String,
}

/// ↑↓ 选择菜单。返回选中项索引，None = 取消。
/// 支持 ↑↓ / j/k / 数字键 / Enter / Esc
pub fn select_menu(items: &[MenuItem]) -> Option<usize> {
    use crossterm::terminal;

    terminal::enable_raw_mode().ok()?;
    let mut selected = 0usize;

    loop {
        // 回到行首
        print!("\r");
        for (i, item) in items.iter().enumerate() {
            let prefix = if i == selected { " >" } else { "  " };
            println!("{} {}. {}", prefix, item.key, item.label);
        }
        print!("\x1b[{}A", items.len()); // 上移 N 行
        stdout().flush().ok()?;

        match event::read() {
            Ok(Event::Key(ke)) if ke.kind == KeyEventKind::Press => match ke.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    selected = selected.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if selected + 1 < items.len() {
                        selected += 1;
                    }
                }
                KeyCode::Enter => {
                    terminal::disable_raw_mode().ok()?;
                    for _ in 0..items.len() {
                        print!("\x1b[K\x1b[1A");
                    }
                    print!("\x1b[J");
                    stdout().flush().ok()?;
                    return Some(selected);
                }
                KeyCode::Esc => {
                    terminal::disable_raw_mode().ok()?;
                    for _ in 0..items.len() {
                        print!("\x1b[K\x1b[1A");
                    }
                    print!("\x1b[J");
                    stdout().flush().ok()?;
                    return None;
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let n = c.to_digit(10).unwrap_or(0) as usize;
                    if n >= 1 && n <= items.len() {
                        selected = n - 1;
                        terminal::disable_raw_mode().ok()?;
                        for _ in 0..items.len() {
                            print!("\x1b[K\x1b[1A");
                        }
                        print!("\x1b[J");
                        stdout().flush().ok()?;
                        return Some(selected);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

/// 普通文本输入
pub fn prompt_input(prompt: &str) -> String {
    use std::io::{stdin, BufRead};
    print!("{} ", prompt);
    stdout().flush().ok();
    let mut line = String::new();
    stdin().lock().read_line(&mut line).ok();
    line.trim().to_string()
}

/// 掩码密码输入
///
/// 注意：当前版本不做字符掩码（终端 echo 正常显示）。
/// 安全场景请使用环境变量配置 API Key。
pub fn prompt_password(prompt: &str) -> String {
    use std::io::{stdin, BufRead};
    print!("{} ", prompt);
    stdout().flush().ok();
    let mut line = String::new();
    stdin().lock().read_line(&mut line).ok();
    println!();
    line.trim().to_string()
}
