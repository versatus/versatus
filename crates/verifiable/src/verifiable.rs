// FEATURE TAG(S): Validator Cores, Tx Validation,
/// A Custom trait that can be implemented on any type that is Verifiable, i.e.
/// Tx, Claim, Block, (Programs?)
pub trait Verifiable {
    type Item;
    type Dependencies;
    type Error;

    fn verifiable(&self) -> bool;

    #[allow(unused_variables)]
    fn valid(
        &self,
        item: &Self::Item,
        debendencies: &Self::Dependencies,
    ) -> Result<bool, Self::Error>;

    #[allow(unused_variables)]
    fn valid_genesis(&self, dependencies: &Self::Dependencies) -> Result<bool, Self::Error> {
        Ok(true)
    }
}
