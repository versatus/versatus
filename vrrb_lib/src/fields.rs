/// A trait that can be implemented on any struct in which you want to know the names of the fields of that struct
pub trait GettableFields {
    fn get_field(&self, field: &str) -> Option<String>;
}
