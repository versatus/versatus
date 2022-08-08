//TODO: Decay is no longer needed, but a tracker measuring the gross utility of the chain is needed so that
// rewards can dynamically adjust. 
pub fn decay_calculator(initial: u128, epochs: u128) -> f64 {
    let b: f64 = 1.0f64 / initial as f64;
    let ln_b = b.log10();
    (ln_b / epochs as f64) * -1.0
}