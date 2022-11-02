pub mod block;
pub mod genesis;
pub mod header;
pub mod invalid;
pub use crate::block::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
