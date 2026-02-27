const RESET: &str = "\x1b[0m";
const NEON_BLUE: &str = "\x1b[38;2;0;180;255m";
const DIM: &str = "\x1b[2m";

const BANNER: &str = r#"
  ███╗   ███╗ █████╗  ██████╗       ███████╗██╗  ██╗███████╗██╗     ██╗
  ████╗ ████║██╔══██╗██╔════╝       ██╔════╝██║  ██║██╔════╝██║     ██║
  ██╔████╔██║███████║██║      █████╗███████╗███████║█████╗  ██║     ██║
  ██║╚██╔╝██║██╔══██║██║      ╚════╝╚════██║██╔══██║██╔══╝  ██║     ██║
  ██║ ╚═╝ ██║██║  ██║╚██████        ███████║██║  ██║███████╗███████╗███████╗
  ╚═╝     ╚═╝╚═╝  ╚═╝ ╚═════╝       ╚══════╝╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝
"#;
const HINT: &str =
    "  Type a command to get started. Use Ctrl+D or 'exit' to quit.\n";

fn get_centered_subtitle() -> String {
    let subtitle = "Go for it!\n";
    let banner_width = 60;
    let padding = (banner_width - subtitle.len()) / 2;
    format!("{:padding$}{}", "", subtitle, padding = padding)
}

pub fn welcome() {
    println!("{NEON_BLUE}{}{RESET}", BANNER);
    println!("{DIM}{:^60}{RESET}", get_centered_subtitle());
    println!("{DIM}{}{RESET}", HINT);
}