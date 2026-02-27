use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, stdout, BufRead, Write};

pub mod commands;
pub mod helpers;

use commands::pwd_state::*;
use crossterm::cursor::{self, MoveToColumn};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use helpers::completion::{complete, last_word};
use helpers::parser::{clear, execute_all, parse_input, ParseResult};
use helpers::welcome::*;

const NEON_BLUE: &str = "\x1b[38;2;0;180;255m";
const RESET: &str    = "\x1b[0m";
const DIM: &str      = "\x1b[2m";

// ─── Persistent history ───────────────────────────────────────────────────────

fn history_path() -> Option<std::path::PathBuf> {
    env::var("HOME").ok().map(|h| std::path::PathBuf::from(h).join(".mac_shell_history"))
}

fn load_history() -> Vec<String> {
    let Some(path) = history_path() else { return vec![] };
    let Ok(file) = fs::File::open(&path) else { return vec![] };
    io::BufReader::new(file)
        .lines()
        .filter_map(|l| l.ok())
        .filter(|l| !l.trim().is_empty())
        .collect()
}

fn append_history(entry: &str) {
    let Some(path) = history_path() else { return };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}", entry);
    }
}

fn push_history(history: &mut Vec<String>, entry: String) {
    if history.last() != Some(&entry) {
        append_history(&entry);
        history.push(entry);
    }
}

// ─── Ctrl+R reverse search ───────────────────────────────────────────────────

/// Find the `skip`-th history entry (from most recent) that contains `query`.
/// Returns the index into `history`.
fn find_in_history(history: &[String], query: &str, skip: usize) -> Option<usize> {
    if query.is_empty() {
        // No query: show the most recent entry offset by skip
        return if skip < history.len() {
            Some(history.len() - 1 - skip)
        } else {
            None
        };
    }
    let mut count = 0usize;
    for (i, entry) in history.iter().enumerate().rev() {
        if entry.contains(query) {
            if count == skip { return Some(i); }
            count += 1;
        }
    }
    None
}

/// Runs an inline Ctrl+R reverse-search sub-loop (we are already in raw mode).
/// Returns the chosen history entry, or None if cancelled.
fn reverse_search(history: &[String]) -> Option<String> {
    let mut query   = String::new();
    let mut skip    = 0usize;   // how many matches to skip (repeated Ctrl+R)

    loop {
        let found_idx = find_in_history(history, &query, skip);
        let display   = found_idx.map(|i| history[i].as_str()).unwrap_or("");

        execute!(stdout(), MoveToColumn(0), Clear(ClearType::CurrentLine)).ok();
        print!("{DIM}(reverse-search){RESET} `{}`: {}", query, display);
        io::stdout().flush().ok();

        let Event::Key(key) = event::read().unwrap() else { continue };
        if key.kind != KeyEventKind::Press { continue; }

        match key.code {
            KeyCode::Enter => {
                print!("\r\n");
                return found_idx.map(|i| history[i].clone());
            }
            KeyCode::Esc => {
                print!("\r\n");
                return None;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                print!("^C\r\n");
                return None;
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Cycle to next (older) match
                skip += 1;
            }
            KeyCode::Backspace => {
                query.pop();
                skip = 0;
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                query.push(c);
                skip = 0;
            }
            _ => {
                // Any other key (arrow, etc.) accepts and exits
                print!("\r\n");
                return found_idx.map(|i| history[i].clone());
            }
        }
    }
}

// ─── Main loop ───────────────────────────────────────────────────────────────

fn main() -> io::Result<()> {
    clear();
    welcome();
    enable_raw_mode()?;

    let mut history:       Vec<String> = load_history();
    let mut input_buffer   = String::new();
    let mut history_index: usize       = history.len();
    let mut input_purline  = String::new();
    let mut is_continuation            = false;

    let start_dir = env::current_dir().expect("Failed to get current working directory");
    let mut pwd_state = PwdState::new(
        start_dir.display().to_string(),
        start_dir.display().to_string(),
    );

    loop {
        let current_display_dir = pwd_state.get_current_dir().replace('\n', "\\n");

        let prompt_len = if is_continuation { 2 }
                         else { current_display_dir.chars().count() + 2 };

        execute!(stdout(), MoveToColumn(0), Clear(ClearType::CurrentLine))?;

        let prompt_text = if !is_continuation {
            format!("{NEON_BLUE}{}$ {RESET}", current_display_dir)
        } else {
            "> ".to_string()
        };

        print!("{}", prompt_text);
        io::stdout().flush()?;

        loop {
            let Event::Key(key_event) = event::read()? else { continue };
            if key_event.kind != KeyEventKind::Press { continue; }

            let (current_x, _) = cursor::position().unwrap();
            let cursor_char_idx = (current_x as usize).saturating_sub(prompt_len);

            match key_event.code {

                // ── Printable character ───────────────────────────────────
                KeyCode::Char(c) => {
                    // Ctrl+D → EOF / exit
                    if key_event.modifiers.contains(KeyModifiers::CONTROL) && c == 'd' {
                        print!("\r\n");
                        disable_raw_mode()?;
                        std::process::exit(0);
                    }
                    // Ctrl+C → cancel current line
                    if key_event.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                        if !input_buffer.trim().is_empty() && !input_buffer.contains('\n') {
                            push_history(&mut history, input_buffer.clone());
                        }
                        history_index = history.len();
                        input_buffer.clear();
                        input_purline.clear();
                        print!("^C\r\n");
                        is_continuation = false;
                        break;
                    }
                    // Ctrl+R → reverse history search
                    if key_event.modifiers.contains(KeyModifiers::CONTROL) && c == 'r' {
                        if let Some(found) = reverse_search(&history) {
                            input_buffer  = found.clone();
                            input_purline = found;
                        }
                        history_index = history.len();
                        // Redraw the prompt with the (possibly updated) buffer
                        execute!(stdout(), MoveToColumn(0), Clear(ClearType::CurrentLine))?;
                        print!("{}{}", prompt_text, input_purline);
                        execute!(stdout(),
                            cursor::MoveToColumn((prompt_len + input_purline.chars().count()) as u16))?;
                        io::stdout().flush()?;
                        continue;
                    }
                    // Ignore other control characters
                    if key_event.modifiers.contains(KeyModifiers::CONTROL) { continue; }

                    // Normal character insert (respects cursor position)
                    if cursor_char_idx >= input_purline.chars().count() {
                        input_buffer.push(c);
                        input_purline.push(c);
                    } else {
                        let byte_idx = input_purline.char_indices()
                            .nth(cursor_char_idx).map(|(i, _)| i)
                            .unwrap_or(input_purline.len());
                        let offset = input_buffer.len() - input_purline.len();
                        input_buffer.insert(offset + byte_idx, c);
                        input_purline.insert(byte_idx, c);
                    }

                    execute!(stdout(),
                        cursor::MoveToColumn(prompt_len as u16),
                        Clear(ClearType::UntilNewLine))?;
                    print!("{}", input_purline);
                    execute!(stdout(),
                        cursor::MoveToColumn((prompt_len + cursor_char_idx + 1) as u16))?;
                    io::stdout().flush()?;
                }

                // ── Tab completion ────────────────────────────────────────
                KeyCode::Tab => {
                    let completions = complete(&input_purline);
                    match completions.len() {
                        0 => { /* nothing — optionally ring bell */ }
                        1 => {
                            // Apply the single completion
                            let word    = last_word(&input_purline);
                            let suffix  = completions[0][word.len()..].to_string();
                            input_buffer.push_str(&suffix);
                            input_purline.push_str(&suffix);

                            execute!(stdout(),
                                cursor::MoveToColumn(prompt_len as u16),
                                Clear(ClearType::UntilNewLine))?;
                            print!("{}", input_purline);
                            execute!(stdout(),
                                cursor::MoveToColumn((prompt_len + input_purline.chars().count()) as u16))?;
                            io::stdout().flush()?;
                        }
                        _ => {
                            // Show all options on the next line, then redraw
                            print!("\r\n{}\r\n", completions.join("  "));
                            execute!(stdout(), MoveToColumn(0), Clear(ClearType::CurrentLine))?;
                            print!("{}{}", prompt_text, input_purline);
                            execute!(stdout(),
                                cursor::MoveToColumn((prompt_len + input_purline.chars().count()) as u16))?;
                            io::stdout().flush()?;
                        }
                    }
                }

                // ── Backspace ─────────────────────────────────────────────
                KeyCode::Backspace => {
                    if !input_purline.is_empty() && cursor_char_idx > 0 {
                        if let Some((idx, _)) = input_purline.char_indices().nth(cursor_char_idx - 1) {
                            let offset = input_buffer.len() - input_purline.len();
                            input_buffer.remove(offset + idx);
                            input_purline.remove(idx);
                        }
                        execute!(stdout(),
                            cursor::MoveToColumn(prompt_len as u16),
                            Clear(ClearType::UntilNewLine))?;
                        print!("{}", input_purline);
                        execute!(stdout(),
                            cursor::MoveToColumn((prompt_len + cursor_char_idx - 1) as u16))?;
                        io::stdout().flush()?;
                    }
                }

                // ── Enter ─────────────────────────────────────────────────
                KeyCode::Enter => {
                    print!("\r\n");
                    io::stdout().flush()?;
                    input_purline.clear();

                    match parse_input(&input_buffer) {
                        ParseResult::Ok(cmds) => {
                            if !input_buffer.trim().is_empty() && !input_buffer.contains('\n') {
                                push_history(&mut history, input_buffer.clone());
                            }
                            history_index = history.len();

                            disable_raw_mode()?;
                            execute_all(cmds, &mut pwd_state);
                            enable_raw_mode()?;

                            input_buffer.clear();
                            is_continuation = false;
                            break;
                        }
                        ParseResult::Incomplete => {
                            input_buffer.push('\n');
                            is_continuation = true;
                            break;
                        }
                    }
                }

                // ── History navigation ────────────────────────────────────
                KeyCode::Up => {
                    if history_index > 0 {
                        history_index -= 1;
                        input_buffer  = history[history_index].clone();
                        input_purline = history[history_index].clone();
                        execute!(stdout(), Clear(ClearType::CurrentLine), MoveToColumn(0))?;
                        print!("{}{}", prompt_text, input_purline.replace('\n', "\r\n"));
                        io::stdout().flush()?;
                    }
                }
                KeyCode::Down => {
                    if history_index < history.len() {
                        history_index += 1;
                        if history_index < history.len() {
                            input_buffer  = history[history_index].clone();
                            input_purline = history[history_index].clone();
                        } else {
                            input_buffer.clear();
                            input_purline.clear();
                        }
                        execute!(stdout(), Clear(ClearType::CurrentLine), MoveToColumn(0))?;
                        print!("{}{}", prompt_text, input_purline.replace('\n', "\r\n"));
                        io::stdout().flush()?;
                    }
                }

                // ── Cursor movement ───────────────────────────────────────
                KeyCode::Left => {
                    if cursor_char_idx > 0 {
                        execute!(stdout(), cursor::MoveToColumn(current_x - 1))?;
                    }
                }
                KeyCode::Right => {
                    if cursor_char_idx < input_purline.chars().count() {
                        execute!(stdout(), cursor::MoveToColumn(current_x + 1))?;
                    }
                }

                _ => {}
            }
        }
    }
}
