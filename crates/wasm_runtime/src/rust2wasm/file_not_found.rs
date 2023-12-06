use std::fs::File;
use std::io::Read;
#[allow(dead_code)]
fn main() {
    let file_path = "non_existent_file.txt";

    let mut file = File::open(file_path).unwrap();

    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    println!("File contents: {}", contents);
}
