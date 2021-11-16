
pub trait Verifiable {
    type Item;
    type DependantOne;
    type DependantTwo;
    type Error;

    fn verifiable(&self) -> bool;

    #[allow(unused_variables)]
    fn valid(
        &self,
        item: &Self::Item,
        dependant_one: &Self::DependantOne,
        dependant_two: &Self::DependantTwo,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }
    
    #[allow(unused_variables)]
    fn valid_genesis(&self, dependant_one: &Self::DependantOne, dependant_two: &Self::DependantTwo) -> Result<bool, Self::Error> {
        Ok(false)
    }
}
