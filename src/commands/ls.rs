use chrono::{DateTime, Duration, Local};
use std::cmp::max;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::time::SystemTime;
use std::{fs, path::Path};
use users::{get_group_by_gid, get_user_by_uid};

/// Active flags for the `ls` command.
#[derive(Debug, Clone, Copy)]
pub struct Flag {
    pub a: bool, // -a : show hidden files
    pub l: bool, // -l : long listing format
    pub f: bool, // -F : append type indicators
}

/// One row of a long listing.
struct LongEntry {
    perms: String,
    links: String,
    user: String,
    group: String,
    size: String,
    date: String,
    name: String,
    blocks: u64,
}

pub fn ls(args: Vec<String>) -> bool {
    let mut flag = Flag { a: false, l: false, f: false };

    let mut files = Vec::new();
    let mut dirs = Vec::new();
    let mut errors = Vec::new();

    // `--` ends flag parsing
    let mut is_dir_marker = false;

    for arg in args {
        if arg == "--" {
            is_dir_marker = true;
            continue;
        }

        if arg.starts_with('-') && !is_dir_marker {
            if !is_flag(&arg, &mut flag) {
                eprintln!("ls: unrecognized option '{}'", arg);
                return false;
            }
            continue;
        }

        let path = Path::new(&arg);

        if path.exists() || fs::symlink_metadata(path).is_ok() {
            if path.is_dir() && !path.is_symlink() {
                dirs.push(arg);
            } else {
                files.push(arg);
            }
        } else {
            errors.push(arg);
        }
    }

    // Default: list current directory
    if files.is_empty() && dirs.is_empty() && errors.is_empty() {
        dirs.push(".".to_string());
    }

    if !list(files, dirs, errors.clone(), flag) || !errors.is_empty() {
        return false;
    }

    true
}

// ─── Core listing logic ───────────────────────────────────────────────────────

fn list(files: Vec<String>, dirs: Vec<String>, errors: Vec<String>, flag: Flag) -> bool {
    for err in &errors {
        eprintln!("ls: cannot access '{}': No such file or directory", err);
    }

    if !files.is_empty() {
        let mut file_entries = Vec::new();
        for file_path in &files {
            let path = Path::new(file_path);
            if let Ok(m) = fs::symlink_metadata(path) {
                if flag.l {
                    file_entries.push(prepare_long_entry(file_path.clone(), &m, flag, path));
                } else {
                    let mut display_name = file_path.clone();
                    if flag.f {
                        display_name = append_indicator(display_name, &m);
                    }
                    println!("{}", display_name);
                }
            }
        }
        if flag.l && !file_entries.is_empty() {
            print!("{}", align_and_format(file_entries, false));
        }
    }

    let show_headers = !files.is_empty() || dirs.len() > 1 || !errors.is_empty();

    for (i, path_str) in dirs.iter().enumerate() {
        if i > 0 || !files.is_empty() {
            println!();
        }
        if show_headers {
            println!("{}:", path_str);
        }

        match (flag.a, flag.l, flag.f) {
            (false, false, false) => {
                match get_dir_content(path_str, false) {
                    Ok(r) => {
                        let s = r.join("  ");
                        if !s.is_empty() { println!("{}", s); }
                    }
                    Err(_) => return false,
                }
            }
            (true, false, false) => {
                match get_dir_content(path_str, true) {
                    Ok(r) => {
                        let s = r.join("  ");
                        if !s.is_empty() { println!("{}", s); }
                    }
                    Err(_) => return false,
                }
            }
            (_, true, _) => {
                print!("{}", run_ls_l(path_str, flag));
            }
            (false, false, true) => {
                match get_dir_content(path_str, false) {
                    Ok(r) => println!("{}", add_symbols(r, path_str)),
                    Err(_) => return false,
                }
            }
            (true, false, true) => {
                match get_dir_content(path_str, true) {
                    Ok(r) => println!("{}", add_symbols(r, path_str)),
                    Err(_) => return false,
                }
            }
        }
    }

    true
}

fn run_ls_l(path: &str, flag: Flag) -> String {
    let mut entries = Vec::new();

    if flag.a {
        // Show . and ..
        if let Ok(metadata) = fs::metadata(path) {
            entries.push(prepare_long_entry(".".into(), &metadata, flag, Path::new(path)));
        }
        let parent = Path::new(path).join("..");
        if let Ok(metadata) = fs::metadata(&parent) {
            entries.push(prepare_long_entry("..".into(), &metadata, flag, &parent));
        }
    }

    if let Ok(read_dir) = fs::read_dir(path) {
        let mut dir_items: Vec<_> = read_dir.filter_map(Result::ok).collect();

        // Sort case-insensitively, stripping punctuation for key comparison
        dir_items.sort_by(|a, b| {
            let name_a = a.file_name().to_string_lossy().to_string();
            let name_b = b.file_name().to_string_lossy().to_string();
            let clean = |s: &str| -> String {
                s.chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>()
                    .to_lowercase()
            };
            let order = clean(&name_a).cmp(&clean(&name_b));
            if order == std::cmp::Ordering::Equal { name_a.cmp(&name_b) } else { order }
        });

        for entry in dir_items {
            let name = entry.file_name().to_string_lossy().to_string();
            if !flag.a && name.starts_with('.') {
                continue;
            }
            if let Ok(metadata) = entry.metadata() {
                entries.push(prepare_long_entry(name, &metadata, flag, &entry.path()));
            }
        }
    }

    align_and_format(entries, true)
}

fn get_dir_content(path: &str, show_hidden: bool) -> Result<Vec<String>, ()> {
    let mut names = Vec::new();

    if show_hidden {
        if fs::metadata(path).is_ok() {
            names.push(".".into());
        }
        let parent = Path::new(path).join("..");
        if fs::metadata(&parent).is_ok() {
            names.push("..".into());
        }
    }

    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries.filter_map(Result::ok) {
                if let Ok(name) = entry.file_name().into_string() {
                    if !show_hidden && name.starts_with('.') {
                        continue;
                    }
                    names.push(name);
                }
            }
        }
        Err(e) => {
            eprintln!("ls: cannot access '{}': {}", path, e);
            return Err(());
        }
    }

    names.sort_by(|a, b| {
        let a_special = a == "." || a == "..";
        let b_special = b == "." || b == "..";
        if a_special && !b_special { return std::cmp::Ordering::Less; }
        if !a_special && b_special { return std::cmp::Ordering::Greater; }
        if a_special && b_special  { return a.cmp(b); }
        let trim = |s: &str| s.trim_start_matches('.').to_lowercase();
        trim(a).cmp(&trim(b))
    });

    Ok(names)
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn is_flag(arg: &str, flag: &mut Flag) -> bool {
    if arg.len() > 1 && arg[1..].chars().all(|c| "alF".contains(c)) {
        for c in arg[1..].chars() {
            match c {
                'a' => flag.a = true,
                'l' => flag.l = true,
                'F' => flag.f = true,
                _   => return false,
            }
        }
        return true;
    }
    false
}

fn add_symbols(paths: Vec<String>, base: &str) -> String {
    paths
        .into_iter()
        .map(|mut name| {
            let full = Path::new(base).join(&name);
            if let Ok(m) = fs::symlink_metadata(&full) {
                let ft = m.file_type();
                if ft.is_dir()         { name.push('/'); }
                else if ft.is_symlink(){ name.push('@'); }
                else if ft.is_fifo()   { name.push('|'); }
                else if ft.is_socket() { name.push('='); }
                else if m.permissions().mode() & 0o111 != 0 { name.push('*'); }
            }
            name
        })
        .collect::<Vec<_>>()
        .join("  ")
}

fn append_indicator(mut name: String, m: &fs::Metadata) -> String {
    if m.is_dir()                            { name.push('/'); }
    else if m.is_symlink()                   { name.push('@'); }
    else if m.file_type().is_fifo()          { name.push('|'); }
    else if m.file_type().is_socket()        { name.push('='); }
    else if m.permissions().mode() & 0o111 != 0 { name.push('*'); }
    name
}

/// Render Unix permission bits as a 10-character string (+ optional `+` for xattrs).
fn format_permissions(m: &fs::Metadata, path: &Path) -> String {
    let mode = m.permissions().mode();
    let mut s = String::with_capacity(11);

    s.push(if m.is_dir()                          { 'd' }
           else if m.is_symlink()                 { 'l' }
           else if m.file_type().is_char_device() { 'c' }
           else if m.file_type().is_block_device(){ 'b' }
           else if m.file_type().is_fifo()        { 'p' }
           else if m.file_type().is_socket()      { 's' }
           else                                   { '-' });

    s.push(if mode & 0o400 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o200 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o4000 != 0 { if mode & 0o100 != 0 { 's' } else { 'S' } }
           else                  { if mode & 0o100 != 0 { 'x' } else { '-' } });

    s.push(if mode & 0o040 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o020 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o2000 != 0 { if mode & 0o010 != 0 { 's' } else { 'S' } }
           else                   { if mode & 0o010 != 0 { 'x' } else { '-' } });

    s.push(if mode & 0o004 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o002 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o1000 != 0 { if mode & 0o001 != 0 { 't' } else { 'T' } }
           else                   { if mode & 0o001 != 0 { 'x' } else { '-' } });

    let has_xattr = xattr::list(path).map(|mut i| i.next().is_some()).unwrap_or(false);
    s.push(if has_xattr { '+' } else { ' ' });

    s
}

/// Format a modification time like `ls -l` does.
fn format_date(modified: SystemTime) -> String {
    let now = SystemTime::now();
    let datetime: DateTime<Local> = modified.into();
    let datetime = datetime + Duration::hours(1);
    let six_months = std::time::Duration::from_secs(180 * 24 * 60 * 60);

    let is_old_or_future = now.duration_since(modified).map(|d| d > six_months).unwrap_or(true);

    if is_old_or_future {
        datetime.format("%b %d  %Y").to_string()
    } else {
        datetime.format("%b %d %H:%M").to_string()
    }
}

/// Produce a column-aligned long listing from a set of entries.
fn align_and_format(entries: Vec<LongEntry>, show_total: bool) -> String {
    if entries.is_empty() {
        return String::new();
    }

    let (mut w_links, mut w_user, mut w_group, mut w_size, mut w_date) = (0, 0, 0, 0, 0);
    let mut total_blocks = 0u64;

    for e in &entries {
        w_links = max(w_links, e.links.len());
        w_user  = max(w_user,  e.user.len());
        w_group = max(w_group, e.group.len());
        w_size  = max(w_size,  e.size.len());
        w_date  = max(w_date,  e.date.len());
        total_blocks += e.blocks;
    }

    let mut out = String::new();
    if show_total {
        out.push_str(&format!("total {}\n", total_blocks / 2));
    }

    for e in entries {
        out.push_str(&format!(
            "{} {:>lw$} {:<uw$} {:<gw$} {:>sw$} {:>dw$} {}\n",
            e.perms,
            e.links,
            e.user,
            e.group,
            e.size,
            e.date,
            e.name,
            lw = w_links,
            uw = w_user,
            gw = w_group,
            sw = w_size,
            dw = w_date,
        ));
    }

    out
}

fn prepare_long_entry(mut name: String, m: &fs::Metadata, flag: Flag, full_path: &Path) -> LongEntry {
    if flag.f && !m.is_symlink() {
        name = append_indicator(name, m);
    }

    // Symlinks: show `name -> target`
    if m.file_type().is_symlink() {
        if let Ok(target) = fs::read_link(full_path) {
            let mut target_str = target.to_string_lossy().to_string();
            if flag.f {
                let resolved = if target.is_absolute() {
                    target.clone()
                } else {
                    full_path.parent().unwrap_or(Path::new(".")).join(&target)
                };
                if let Ok(tm) = fs::metadata(&resolved) {
                    target_str = append_indicator(target_str, &tm);
                }
            }
            name.push_str(" -> ");
            name.push_str(&target_str);
        }
    }

    let perms = format_permissions(m, full_path);
    let links = m.nlink().to_string();

    let user = get_user_by_uid(m.uid())
        .map(|u| u.name().to_string_lossy().to_string())
        .unwrap_or_else(|| m.uid().to_string());

    let group = get_group_by_gid(m.gid())
        .map(|g| g.name().to_string_lossy().to_string())
        .unwrap_or_else(|| m.gid().to_string());

    let size = if m.file_type().is_block_device() || m.file_type().is_char_device() {
        let rdev = m.rdev();
        format!("{:>3}, {:>3}", libc::major(rdev), libc::minor(rdev))
    } else {
        m.len().to_string()
    };

    let date = format_date(m.modified().unwrap_or(SystemTime::now()));

    LongEntry { perms, links, user, group, size, date, name, blocks: m.blocks() }
}
