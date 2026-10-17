fn main() {
    if let Err(e) = zov::run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
