#[macro_export]
macro_rules! create_payload {
    ($($x:expr),*) => {{
        let mut payload = String::new();

        $(
            payload.push_str(&format!("{:?}", $x));
        )*

        Message::from(s256::Hash::hash(&payload.as_bytes()))
    }};
}

#[macro_export]
macro_rules! hash_data {
    ($($item:expr),+) => {{
        use sha2::{Digest, Sha256};
        use serde::{de::DeserializeOwned, Serialize};
        use serde_json::to_vec;

        fn update_hasher_with_item<T: Serialize + DeserializeOwned>(
            hasher: &mut Sha256, item: &T
        ) {
            let serialized = serde_json::to_vec(item).unwrap();
            hasher.update(&serialized);
        }

        let mut hasher = Sha256::new();
        $(
            update_hasher_with_item(&mut hasher, &$item);
        )+

        hasher.finalize()
    }};
}

