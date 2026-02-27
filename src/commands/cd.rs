use crate::commands::pwd_state::PwdState;
use std::{env, io::ErrorKind, path::PathBuf};
use libc;

/// Change directory.
///
/// - No args          → $HOME
/// - `cd -`           → previous directory (prints it)
/// - `cd ~`           → $HOME
/// - `cd <path>`      → target path
///
/// `error_path` holds the raw (un-sanitised) path tokens for user-facing
/// error messages; `args` holds the sanitised tokens used for actual logic.
pub fn command_cd(
    error_path: Vec<String>, // raw tokens for error messages
    mut args: Vec<String>,   // sanitised tokens for logic
    pwd_state: &mut PwdState,
) -> bool {
    if args.len() > 1 {
        eprintln!("cd: too many arguments");
        return false;
    }

    // Unescape literal \n sequences in the path argument
    if args.len() == 1 {
        args[0] = args[0].replace("\\n", "\n");
    }

    let target_dir = if args.is_empty() {
        // cd with no args → HOME
        match env::var("HOME") {
            Ok(path) => PathBuf::from(path),
            Err(_) => {
                eprintln!("cd: HOME environment variable not set");
                return false;
            }
        }
    } else if args[0] == "-" {
        // cd - → previous directory
        PathBuf::from(pwd_state.get_old_dir())
    } else if args[0] == "~" {
        // cd ~ → HOME
        match env::var("HOME") {
            Ok(path) => PathBuf::from(path),
            Err(_) => {
                eprintln!("cd: HOME environment variable not set");
                return false;
            }
        }
    } else {
        PathBuf::from(&args[0])
    };

    let current_before_move = pwd_state.get_current_dir();

    match env::set_current_dir(&target_dir) {
        Ok(_) => {
            if let Ok(new_current) = env::current_dir() {
                pwd_state.set_states(new_current.display().to_string(), current_before_move);
            } else {
                pwd_state.set_states(
                    PathBuf::from(".").display().to_string(),
                    current_before_move,
                );
            }

            // cd - prints the new directory
            if !args.is_empty() && args[0] == "-" {
                println!("{}", pwd_state.get_current_dir());
            }

            true
        }
        Err(e) => {
            let display = if error_path.is_empty() {
                args.first().map(|s| s.as_str()).unwrap_or("")
            } else {
                &error_path[0]
            };

            match e.kind() {
                ErrorKind::NotFound => {
                    eprintln!("cd: No such file or directory: {}", display);
                }
                ErrorKind::PermissionDenied => {
                    eprintln!("cd: Permission denied: {}", display);
                }
                ErrorKind::Other | _ if e.raw_os_error() == Some(libc::ENOTDIR) => {
                    eprintln!("cd: Not a directory: {}", display);
                }
                _ => {
                    eprintln!("cd: {}: {}", display, e);
                }
            }
            false
        }
    }
}
