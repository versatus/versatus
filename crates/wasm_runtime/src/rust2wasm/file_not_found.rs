use std::fs::File;
#[allow(dead_code)]
fn main() {
    let file_path = "non_existent_file.txt";

    let mut file = File::open(file_path).unwrap();
}
