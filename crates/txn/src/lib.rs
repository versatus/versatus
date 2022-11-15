pub mod txn;

#[cfg(test)]
mod tests {

    use crate::txn::*;

    #[test]
    fn convert_obols_to_vrrb_successfully() {
        assert_eq!(Vrrb(10), Obol(10 * OBOLS_IN_VRRB).try_into().unwrap())
    }
    #[test]
    fn convert_vrrb_to_obol_successfully() {
        assert_eq!(Obol(10 * OBOLS_IN_VRRB), Vrrb(10).try_into().unwrap())
    }
    #[test]
    fn fail_conversion_obols_to_vrrb() {
        assert_eq!(
            Vrrb::try_from(Obol((1.5 * OBOLS_IN_VRRB as f32) as u128)),
            Err(SystemTokenError::ConversionError)
        );
    }

    #[test]
    fn fail_conversion_vrrb_to_obol() {
        assert_eq!(
            Obol::try_from(Vrrb(u128::MAX)),
            Err(SystemTokenError::ConversionError)
        );
    }
}
