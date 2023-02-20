use ritelinked::LinkedHashMap;
use sha256::Sha256Digest;

use crate::txn::Txn;

pub fn gen_sha256_digest_string<D: Sha256Digest>(data: D) -> String {
    sha256::digest(data)
}

#[macro_export]
macro_rules! is_enum_variant {
    ($v:expr, $p:pat) => {
        if let $p = $v {
            true
        } else {
            false
        }
    };
}

pub fn size_of_txn_list(txns: &LinkedHashMap<String, Txn>) -> usize {
    txns.iter()
        .map(|(_, set)| set)
        .map(|txn| std::mem::size_of_val(&txn))
        .sum()
}
