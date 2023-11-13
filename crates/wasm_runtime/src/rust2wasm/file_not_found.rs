#![allow(dead_code)]

use std::io;
fn main() -> io::Result<()> {
    Ok({std::fs::File::open("fake.txt")?;})
}
