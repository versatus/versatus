pub trait GettableFields {
    fn get_field(&self, field: &str) -> Option<String>;
}
