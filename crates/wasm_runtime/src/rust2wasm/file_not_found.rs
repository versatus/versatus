#![allow(dead_code)]
use std::fs::File;

fn main() {
    let ecode = real_main().err().unwrap().raw_os_error().unwrap();
    std::process::exit(ecode);
}

fn real_main() -> Result<File, std::io::Error> {
    std::fs::File::open("fake.txt")
}
