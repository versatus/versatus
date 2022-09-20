/// A trait that can be implemented on any type that can be updated.
pub trait Updateable {
    type Input;
    fn update(&mut self, category: Self::Input);
}
