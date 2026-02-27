use std::path::Path;

/// Remove files or directories.
/// Flags: -r / -R / --recursive  → remove directories recursively.
pub fn rm(args: Vec<String>) -> bool {
    let mut recursive = false;

    for arg in &args {
        if arg == "--recursive" || arg == "-r" || arg == "-R" || arg == "-rf" || arg == "-fr" {
            recursive = true;
            continue;
        }
        if arg.starts_with('-') {
            for c in arg[1..].chars() {
                match c {
                    'r' | 'R' | 'f' => recursive = true,
                    _ => {
                        eprintln!("rm: invalid option -- '{}'", c);
                        return false;
                    }
                }
            }
        }
    }

    let targets: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();

    if targets.is_empty() {
        eprintln!("rm: missing operand");
        return false;
    }

    let mut all_ok = true;

    for arg in targets {
        let path = Path::new(arg);

        // Refuse to remove . or ..
        if matches!(
            path.file_name().and_then(|n| n.to_str()),
            Some(".") | Some("..")
        ) {
            eprintln!("rm: refusing to remove '.' or '..' directory: skipping '{}'", arg);
            all_ok = false;
            continue;
        }

        match std::fs::symlink_metadata(path) {
            Ok(meta) => {
                if meta.is_symlink() {
                    if let Err(e) = std::fs::remove_file(path) {
                        eprintln!("rm: cannot remove symlink '{}': {}", arg, e);
                        all_ok = false;
                    }
                } else if meta.is_dir() {
                    if !recursive {
                        eprintln!("rm: cannot remove '{}': Is a directory", arg);
                        all_ok = false;
                    } else if let Err(e) = std::fs::remove_dir_all(path) {
                        eprintln!("rm: cannot remove '{}': {}", arg, e);
                        all_ok = false;
                    }
                } else if let Err(e) = std::fs::remove_file(path) {
                    eprintln!("rm: cannot remove '{}': {}", arg, e);
                    all_ok = false;
                }
            }
            Err(e) => {
                eprintln!("rm: cannot remove '{}': {}", arg, e);
                all_ok = false;
            }
        }
    }

    all_ok
}
