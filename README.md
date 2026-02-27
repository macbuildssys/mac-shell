```
  ███╗   ███╗ █████╗  ██████╗       ███████╗██╗  ██╗███████╗██╗     ██╗
  ████╗ ████║██╔══██╗██╔════╝       ██╔════╝██║  ██║██╔════╝██║     ██║
  ██╔████╔██║███████║██║      █████╗███████╗███████║█████╗  ██║     ██║
  ██║╚██╔╝██║██╔══██║██║      ╚════╝╚════██║██╔══██║██╔══╝  ██║     ██║
  ██║ ╚═╝ ██║██║  ██║╚██████        ███████║██║  ██║███████╗███████╗███████╗
  ╚═╝     ╚═╝╚═╝  ╚═╝ ╚═════╝       ╚══════╝╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝
                                    Go for it!
```

# mac-shell

An interactive, minimalist Unix-like shell built in Rust. Explore how shells work under the hood in a simple, approachable way.


## Features

### Input Parsing

- Single quotes `'...'`, double quotes `"..."`, and backslash escaping `\`

- Command chaining with `;` (unconditional), `&&` (on success), and `&` (always continue)

- Multi-line input; incomplete quotes or escapes prompt a `>` continuation line


### Built-in Commands
| Command | Description |
|---------|-------------|
| `cd [path]` | Change directory; `cd -` returns to previous directory; `cd ~` goes home |
| `pwd` | Print working directory |
| `ls [-a] [-l] [-F]` | List directory contents with optional flags |
| `cat [file...]` | Concatenate files; no arguments reads from stdin |
| `cp <src> <dest>` | Copy files |
| `mv <src> <dest>` | Move or rename files |
| `rm [-r] <path>` | Remove files or directories |
| `mkdir <dir>` | Create a directory |
| `echo [args...]` | Print arguments to stdout |
| `clear` | Clear the terminal screen |
| `exit` | Exit the shell |


### External Commands

Any command not listed above is looked up on `$PATH` (including `/usr/sbin`, `/sbin`, and other system directories) and spawned as a child process; so tools like `sudo`, `git`, `python3`, `ifconfig`, `apt-get`, and anything else installed on your system work out of the box.


### Interactive CLI

- **Persistent history**: Commands are saved to `~/.mac_shell_history` and loaded on startup

- **History navigation**: `↑` / `↓` arrows cycle through previous commands

- **Ctrl+R**: Reverse incremental history search; type to filter; press Ctrl+R again to cycle older matches; Enter to confirm; Esc to cancel

- **Tab completion**: Completes command names (from PATH + builtins) at the start of a line; completes filesystem paths for arguments; appends `/` to directories automatically; prints all matches when ambiguous

- **Inline cursor editing**: `←` / `→` to move within the current line; Backspace deletes at cursor
- **Ctrl+C**: Cancel the current input line

- **Ctrl+D or exit**: Exit the shell


## Installation

### Prerequisites

- [Rust](https://rustup.rs/) 1.70 or later

### Build from source

```
git clone https://github.com/macbuildssys/mac-shell.git

cd mac-shell

cargo build --release
```

The compiled binary will be at `./target/release/mac-shell`

### Run

```
./target/release/mac-shell
```

Or build and run in one step:

```
cargo run --release
```

### Optional; add to PATH

```
cp target/release/mac-shell /usr/local/bin/mac-shell
```


## Design Decisions

**Raw mode input**: Via `crossterm`: the shell owns the terminal directly rather than relying on the OS line-discipline, which is what enables inline cursor movement, Tab completion, and history navigation.

**No `fork`/`exec` directly**: Rust's `std::process::Command` is used for spawning external processes. It wraps the underlying `fork`+`exec` syscalls safely and is the idiomatic Rust approach.

**State in `PwdState`**: The current and previous working directory are tracked in a struct rather than relying solely on `std::env::set_current_dir`, which enables `cd -` to work correctly across the shell's lifetime.

**Augmented PATH**: When looking up external commands, mac-shell merges the user's `$PATH` with system directories (`/usr/sbin`, `/sbin`, etc.) that are often absent from interactive login PATHs. This means tools like `ifconfig` and `ip` work without any manual PATH configuration.


## Dependencies

| Crate | Purpose |
|-------|---------|
| [`crossterm`](https://crates.io/crates/crossterm) | Cross-platform raw terminal input; cursor control; screen clearing |
| [`chrono`](https://crates.io/crates/chrono) | Date formatting for `ls -l` timestamps |
| [`users`](https://crates.io/crates/users) | Resolving UID/GID to usernames for `ls -l` |
| [`xattr`](https://crates.io/crates/xattr) | Detecting extended attributes for `ls -l` permission display |
| [`libc`](https://crates.io/crates/libc) | Raw OS error codes; block/char device major/minor numbers |


## License

Distributed under the MIT License. See [LICENSE](LICENSE).