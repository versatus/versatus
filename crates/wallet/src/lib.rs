mod v1;
pub mod v2;

#[deprecated(note = "This module will be migrated to v2 soon which includes breaking changes")]
pub mod wallet {
    #[deprecated]
    pub use crate::v1::*;
}
