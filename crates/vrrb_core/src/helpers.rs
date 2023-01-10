use primitives::{ByteVec, Digest};
use sha2::Sha256;
use sha256::Sha256Digest;

pub fn gen_sha256_digest_string<D: Sha256Digest>(data: D) -> String {
    sha256::digest(data)
}
