pub mod payload;

pub mod time {
    #[macro_export]
    /// Returns a Unix timestamp in UTC
    macro_rules! timestamp {
        () => {{
            chrono::offset::Utc::now().timestamp()
        }};
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
