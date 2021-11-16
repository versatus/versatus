pub trait Updateable {
    type Input;
    fn update(&mut self, category: Self::Input);
}