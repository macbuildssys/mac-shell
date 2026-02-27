/// Print arguments to stdout separated by spaces, followed by a newline.
pub fn echo(args: Vec<String>) {
    println!("{}", args.join(" "));
}
