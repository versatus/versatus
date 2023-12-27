#[allow(dead_code)]
fn main() {
    std::fs::File::open("non_existent_file.txt").unwrap();
}
