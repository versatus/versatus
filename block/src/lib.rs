pub mod block;
pub mod header;
pub mod invalid;
pub use block::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
