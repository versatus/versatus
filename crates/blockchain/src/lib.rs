pub mod blockchain;

#[cfg(test)]
mod tests {
    use super::blockchain::*;

    #[test]
    fn it_works() {
        let mut chain = Blockchain::new("bananas");
        assert_eq!(2 + 2, 4);
    }
}
