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
// This allow statement is **not** part of the code used in
// creating the relative .wasm file and is only used here
// to dismiss clippy warnings.
#[allow(unconditional_recursion)]
fn add_plate(stack: &mut Vec<Plate>) {
    stack.push(Plate::new());
    add_plate(stack)
}
