pub type NodeId = u16;
pub type SecretKey = Vec<u8>;



//TXN Hash or Block Hash
pub type Hash = Vec<u8>;
pub type RawSignature = Vec<u8>;

pub enum SignatureType{
    PartialSignature,
    ThresholdSignature,
    ChainLockSignature
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



#[cfg(test)]
mod tests {
    use super::*;
}
