fn main() {
    infinite_recursion()
}

#[allow(unconditional_recursion)]
fn infinite_recursion() {
    infinite_recursion()
}
