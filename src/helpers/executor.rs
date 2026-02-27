use crate::commands::{
    cat::cat, cd::command_cd, cp::cp, echo::echo, exit::exit,
    ls::ls, mv::mv, pwd_state::PwdState, rm::rm,
};
use super::parser::CommandEnum;

/// Dispatch a parsed command to its handler.
/// Returns true if execution succeeded.
pub fn execute(cmd: CommandEnum, pwd_state: &mut PwdState) -> bool {
    match cmd {
        CommandEnum::Ls(args)  => ls(args),
        CommandEnum::Mv(args)  => mv(args),
        CommandEnum::Cp(args)  => cp(args),
        CommandEnum::Cat(args) => cat(args),

        CommandEnum::Rm(args) => {
            if args.is_empty() { eprintln!("rm: missing operand"); return false; }
            rm(args)
        }

        CommandEnum::Mkdir(raw_dirs, sanitised_dirs) => {
            if raw_dirs.is_empty() { eprintln!("mkdir: missing operand"); return false; }
            for (i, dir) in raw_dirs.iter().enumerate() {
                if let Err(e) = std::fs::create_dir(dir) {
                    eprintln!("mkdir: cannot create directory '{}': {}",
                        sanitised_dirs.get(i).map(|s| s.as_str()).unwrap_or(dir), e);
                    return false;
                }
            }
            true
        }

        CommandEnum::Pwd => {
            println!("{}", pwd_state.get_current_dir().replace('\n', "\\n"));
            true
        }

        CommandEnum::Cd(sanitised_args, raw_args) => {
            command_cd(raw_args, sanitised_args, pwd_state)
        }

        CommandEnum::Echo(args) => { echo(args); true }

        CommandEnum::Exit => exit(),

        CommandEnum::Unknown(cmd, args) => {
            // Spawn as external process. Augment PATH with common sbin locations
            // so tools like ifconfig, ip, etc. are found even if the user's login
            // PATH doesn't include them.
            let augmented_path = {
                let base = std::env::var("PATH").unwrap_or_default();
                let extra = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";
                // Only add dirs not already in PATH to avoid duplicates.
                let additions: Vec<&str> = extra.split(':')
                    .filter(|d| !base.split(':').any(|b| b == *d))
                    .collect();
                if additions.is_empty() {
                    base
                } else {
                    format!("{}:{}", base, additions.join(":"))
                }
            };

            match std::process::Command::new(&cmd)
                .args(&args)
                .env("PATH", &augmented_path)
                .status()
            {
                Ok(status) => status.success(),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    eprintln!("command not found: {}", cmd.replace('\n', "\\n"));
                    false
                }
                Err(e) => {
                    eprintln!("{}: {}", cmd.replace('\n', "\\n"), e);
                    false
                }
            }
        }
    }
}
