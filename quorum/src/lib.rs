pub  mod election;
pub mod quorum;
pub mod dummyNode;
extern crate snowflake;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
