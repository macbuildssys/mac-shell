use super::executor::execute;
use crate::commands::pwd_state::PwdState;

/// All supported built-in commands, carrying their parsed arguments.
#[derive(Debug)]
pub enum CommandEnum {
    Rm(Vec<String>),
    Cp(Vec<String>),
    Mv(Vec<String>),
    Pwd,
    Cd(Vec<String>, Vec<String>),
    Echo(Vec<String>),
    Mkdir(Vec<String>, Vec<String>),
    Exit,
    /// An unrecognised built-in: spawn as an external process.
    Unknown(String, Vec<String>),
    Cat(Vec<String>),
    Ls(Vec<String>),
}

/// Result of parsing. Ok carries Vec<(CommandEnum, bool)> where bool =
/// requires_prev_success: false=always run, true=only if prev succeeded (&&).
#[derive(Debug)]
pub enum ParseResult {
    Ok(Vec<(CommandEnum, bool)>),
    Incomplete,
}

fn parse_tokens(input: &str) -> Result<Vec<(Vec<String>, bool)>, String> {
    let mut commands: Vec<(Vec<String>, bool)> = Vec::new();
    let mut current_args: Vec<String> = Vec::new();
    let mut current_token = String::new();
    let mut next_req: bool = false;

    #[derive(Clone, Copy, PartialEq)]
    enum Mode { Normal, Single, Double }

    let mut mode = Mode::Normal;
    let mut escaped = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if escaped {
            match mode {
                Mode::Normal => current_token.push(c),
                Mode::Double => {
                    if c == '"' || c == '\\' { current_token.push(c); }
                    else { current_token.push('\\'); current_token.push(c); }
                }
                Mode::Single => { current_token.push('\\'); current_token.push(c); }
            }
            escaped = false;
            continue;
        }

        match mode {
            Mode::Normal => match c {
                '\\' => escaped = true,
                '\'' => mode = Mode::Single,
                '"'  => mode = Mode::Double,
                // `;` → unconditional separator, always run next command
                ';' => {
                    flush_token(&mut current_token, &mut current_args);
                    flush_args(&mut current_args, &mut commands, next_req);
                    next_req = false;
                }
                '&' => {
                    if chars.peek() == Some(&'&') {
                        chars.next();
                        flush_token(&mut current_token, &mut current_args);
                        flush_args(&mut current_args, &mut commands, next_req);
                        next_req = true;
                    } else {
                        // single & → always run next command
                        flush_token(&mut current_token, &mut current_args);
                        flush_args(&mut current_args, &mut commands, next_req);
                        next_req = false;
                    }
                }
                c if c.is_whitespace() => flush_token(&mut current_token, &mut current_args),
                c => current_token.push(c),
            },
            Mode::Single => {
                if c == '\'' { mode = Mode::Normal; } else { current_token.push(c); }
            }
            Mode::Double => match c {
                '\\' => escaped = true,
                '"'  => mode = Mode::Normal,
                c    => current_token.push(c),
            },
        }
    }

    if escaped || mode != Mode::Normal {
        return Err("Incomplete".into());
    }

    flush_token(&mut current_token, &mut current_args);
    flush_args(&mut current_args, &mut commands, next_req);

    Ok(commands)
}

#[inline]
fn flush_token(token: &mut String, args: &mut Vec<String>) {
    if !token.is_empty() { args.push(std::mem::take(token)); }
}

#[inline]
fn flush_args(args: &mut Vec<String>, commands: &mut Vec<(Vec<String>, bool)>, req: bool) {
    if !args.is_empty() { commands.push((std::mem::take(args), req)); }
}

pub fn parse_input(input: &str) -> ParseResult {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return ParseResult::Ok(vec![]);
    }

    match parse_tokens(trimmed) {
        Err(_) => ParseResult::Incomplete,
        Ok(tokenized) => {
            let mut cmds: Vec<(CommandEnum, bool)> = Vec::new();

            for (args, req) in tokenized {
                if args.is_empty() { continue; }

                let cmd_name = args[0].as_str();
                let raw_args  = args[1..].to_vec();
                let sanitised: Vec<String> = raw_args.iter()
                    .map(|s| s.replace('\n', "\\n"))
                    .collect();

                let parsed = match cmd_name {
                    "ls"    => CommandEnum::Ls(sanitised),
                    "cat"   => CommandEnum::Cat(sanitised),
                    "cp"    => CommandEnum::Cp(sanitised),
                    "pwd"   => CommandEnum::Pwd,
                    "cd"    => CommandEnum::Cd(sanitised, raw_args),
                    "echo"  => CommandEnum::Echo(raw_args),
                    "rm"    => CommandEnum::Rm(sanitised),
                    "mkdir" => CommandEnum::Mkdir(raw_args, sanitised),
                    "mv"    => CommandEnum::Mv(sanitised),
                    "exit"  => CommandEnum::Exit,
                    "clear" => { clear(); continue; }
                    _       => CommandEnum::Unknown(args[0].clone(), raw_args),
                };

                cmds.push((parsed, req));
            }

            ParseResult::Ok(cmds)
        }
    }
}

/// Execute commands, honouring && short-circuit. & groups always run.
pub fn execute_all(cmds: Vec<(CommandEnum, bool)>, pwd_state: &mut PwdState) {
    let mut last_ok = true;
    for (cmd, requires_prev_success) in cmds {
        if requires_prev_success && !last_ok {
            return;
        }
        last_ok = execute(cmd, pwd_state);
    }
}

pub fn clear() {
    print!("\x1Bc");
}
