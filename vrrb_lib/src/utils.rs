use pickledb::{PickleDb, PickleDbDumpPolicy, SerializationMethod};

pub fn decay_calculator(initial: u128, epochs: u128) -> f64 {
    let b: f64 = 1.0f64 / initial as f64;
    let ln_b = b.log10();
    (ln_b / epochs as f64) * -1.0
}

pub fn restore_db(path: &str) -> PickleDb {
    let db = match PickleDb::load_bin(path, PickleDbDumpPolicy::DumpUponRequest) {
        Ok(nst) => nst,
        Err(_) => PickleDb::new(
            path,
            PickleDbDumpPolicy::DumpUponRequest,
            SerializationMethod::Bin,
        )};
    db
}