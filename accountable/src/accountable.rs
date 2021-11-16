pub trait Accountable {
    type Category;

    fn receivable(&self) -> String;
    fn payable(&self) -> Option<String>;
    fn get_amount(&self) -> u128;
    fn get_category(&self) -> Option<Self::Category>;
}