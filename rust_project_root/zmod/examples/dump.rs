//! `cargo run --example dump` — list what `$sources/lib` currently offers.
//!
//! Same code path the shell uses on its refresh tick, so this is the quickest
//! way to tell "my module is not loading" apart from "my module is loading and
//! the card is binding the wrong key".

fn main() {
    let Some(dir) = zmod::lib_dir() else {
        eprintln!("no $sources: set `sources`, or `states2` next to it, or run with HOME set");
        std::process::exit(1);
    };
    println!("$sources/lib = {}", dir.display());
    let reg = zmod::load_dir(&dir);
    print!("{}", reg.status());
    if !reg.errors.is_empty() {
        std::process::exit(1);
    }
}
