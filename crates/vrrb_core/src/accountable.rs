/// A trait that enables the type it is implemented on to provide some basic
/// accounting functionality.
pub trait Accountable {
    type Category;

    fn receivable(&self) -> String;
    fn payable(&self) -> Option<String>;
    fn get_amount(&self) -> u128;
    fn get_category(&self) -> Option<Self::Category>;
}
