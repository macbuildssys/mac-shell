use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::enable_raw_mode,
};
use std::{
    fs::File,
    io::{self, Write},
    path::Path,
};

/// Concatenate and print files to stdout.
/// With no arguments, reads from stdin in raw mode (like real cat).
pub fn cat(args: Vec<String>) -> bool {
    let mut error_count = 0;
    let stdout = io::stdout();

    if args.is_empty() {
        // No files: enter raw-mode stdin loop, echo lines back on Enter.
        match enable_raw_mode() {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Failed to enable raw mode: {}", e);
                return false;
            }
        }

        let mut input_buffer = String::new();

        loop {
            if let Event::Key(key_event) = event::read().unwrap() {
                if key_event.kind == KeyEventKind::Press {
                    match key_event.code {
                        // Ctrl+D → EOF
                        KeyCode::Char('d')
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            print!("\r\n");
                            break;
                        }
                        // Ctrl+C → interrupt
                        KeyCode::Char('c')
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            print!("^C\r\n");
                            break;
                        }
                        KeyCode::Char(c) => {
                            print!("{}", c);
                            input_buffer.push(c);
                            io::stdout().flush().ok();
                        }
                        KeyCode::Backspace => {
                            if !input_buffer.is_empty() {
                                input_buffer.pop();
                                // Move back, overwrite with space, move back again
                                print!("\x08 \x08");
                                io::stdout().flush().ok();
                            }
                        }
                        KeyCode::Enter => {
                            // Echo the buffered line back (mimics real cat)
                            print!("\r\n{}\r\n", input_buffer);
                            io::stdout().flush().ok();
                            input_buffer.clear();
                        }
                        _ => {}
                    }
                }
            }
        }
    } else {
        // Print each file in order
        for file in args {
            let source_path = Path::new(&file);
            match File::open(source_path) {
                Ok(mut f) => match io::copy(&mut f, &mut stdout.lock()) {
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("cat: {}: {}", file, e);
                        error_count += 1;
                    }
                },
                Err(e) => {
                    eprintln!("cat: {}: {}", file, e);
                    error_count += 1;
                }
            }
        }
    }

    error_count == 0
}
