use std::fs;
use std::path::Path;

/// Move / rename files.
/// Usage: mv SOURCE DEST  |  mv SOURCE... DIRECTORY
pub fn mv(args: Vec<String>) -> bool {
    if args.len() < 2 {
        eprintln!("mv: missing operand");
        return false;
    }

    if args.len() == 2 {
        let src = &args[0];
        let dst = &args[1];
        let src_path = Path::new(src);
        let dst_path = Path::new(dst);

        // If destination is a directory, move source inside it
        let final_dst = if dst_path.is_dir() {
            match src_path.file_name() {
                Some(name) => dst_path.join(name),
                None => dst_path.to_path_buf(),
            }
        } else {
            dst_path.to_path_buf()
        };

        if let Err(e) = fs::rename(src_path, &final_dst) {
            eprintln!("mv: cannot move '{}': {}", src, e);
        }
        return true;
    }

    // Multiple sources → last argument must be a directory
    let dst_dir = Path::new(args.last().unwrap());
    if !dst_dir.is_dir() {
        eprintln!("mv: target '{}' is not a directory", dst_dir.display());
        return false;
    }

    for src in &args[0..args.len() - 1] {
        let src_path = Path::new(src);
        if let Some(file_name) = src_path.file_name() {
            let dst = dst_dir.join(file_name);
            if let Err(e) = fs::rename(src_path, &dst) {
                eprintln!("mv: cannot move '{}': {}", src, e);
            }
        }
    }

    true
}
