use std::io::{self, Read};

fn main() {
    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    handle.read_to_string(&mut buffer).expect("failed to io::read");

    println!("{}", buffer);
}