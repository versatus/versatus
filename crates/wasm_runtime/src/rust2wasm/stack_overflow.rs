#![allow(dead_code)]
struct Plate;
impl Plate {
    fn new() -> Plate {
        Plate
    }
}
fn main() {
    let mut stack = Vec::new();
    add_plate(&mut stack)
}
fn add_plate(stack: &mut Vec<Plate>) {
    stack.push(Plate::new());
    add_plate(stack)
}
