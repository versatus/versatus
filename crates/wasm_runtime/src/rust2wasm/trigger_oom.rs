#![allow(dead_code)]

fn trigger_oom() {
    let mut v: Vec<i32> = Vec::new();

    let huge_value = i32::MAX;

    loop {
        v.push(huge_value);
    }
}

fn main() {
    trigger_oom();
}
