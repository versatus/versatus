use hex::FromHex;
use rand::Rng;
use std::sync::Arc;

use lr_trie::db::MemoryDB;
use lr_trie::inner::InnerTrie;
use lr_trie::trie::Trie;

fn assert_root(data: Vec<(&[u8], &[u8])>, hash: &str) {
    let memdb = Arc::new(MemoryDB::new(true));
    let mut trie = InnerTrie::new(Arc::clone(&memdb));
    for (k, v) in data.into_iter() {
        trie.insert(k, v).unwrap();
    }
    let root_hash = trie.root_hash().unwrap();
    let rs = format!("0x{}", hex::encode(root_hash));
    assert_eq!(rs.as_str(), hash);

    let mut trie = trie.at_root(root_hash);
    let r2 = trie.root_hash().unwrap();
    let rs2 = format!("0x{}", hex::encode(r2));
    assert_eq!(rs2.as_str(), hash);
}

#[test]
fn test_root() {
    // See: https://github.com/ethereum/tests/blob/develop/TrieTests
    // Copy from trietest.json and trieanyorder.json
    assert_root(
        vec![(b"A", b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")],
        "0xd23786fb4a010da3ce639d66d5e904a11dbc02746d1ce25029e53290cabf28ab",
    );
    assert_root(
        vec![
            (b"doe", b"reindeer"),
            (b"dog", b"puppy"),
            (b"dogglesworth", b"cat"),
        ],
        "0x8aad789dff2f538bca5d8ea56e8abe10f4c7ba3a5dea95fea4cd6e7c3a1168d3",
    );
    assert_root(
        vec![
            (b"do", b"verb"),
            (b"horse", b"stallion"),
            (b"doge", b"coin"),
            (b"dog", b"puppy"),
        ],
        "0x5991bb8c6514148a29db676a14ac506cd2cd5775ace63c30a4fe457715e9ac84",
    );
    assert_root(
        vec![(b"foo", b"bar"), (b"food", b"bass")],
        "0x17beaa1648bafa633cda809c90c04af50fc8aed3cb40d16efbddee6fdf63c4c3",
    );

    assert_root(
        vec![(b"be", b"e"), (b"dog", b"puppy"), (b"bed", b"d")],
        "0x3f67c7a47520f79faa29255d2d3c084a7a6df0453116ed7232ff10277a8be68b",
    );
    assert_root(
        vec![(b"test", b"test"), (b"te", b"testy")],
        "0x8452568af70d8d140f58d941338542f645fcca50094b20f3c3d8c3df49337928",
    );
    assert_root(
        vec![
            (
                Vec::from_hex("0045").unwrap().as_slice(),
                Vec::from_hex("0123456789").unwrap().as_slice(),
            ),
            (
                Vec::from_hex("4500").unwrap().as_slice(),
                Vec::from_hex("9876543210").unwrap().as_slice(),
            ),
        ],
        "0x285505fcabe84badc8aa310e2aae17eddc7d120aabec8a476902c8184b3a3503",
    );
    assert_root(
        vec![
            (b"do", b"verb"),
            (b"ether", b"wookiedoo"),
            (b"horse", b"stallion"),
            (b"shaman", b"horse"),
            (b"doge", b"coin"),
            (b"ether", b""),
            (b"dog", b"puppy"),
            (b"shaman", b""),
        ],
        "0x5991bb8c6514148a29db676a14ac506cd2cd5775ace63c30a4fe457715e9ac84",
    );
    assert_root(
        vec![
            (b"do", b"verb"),
            (b"ether", b"wookiedoo"),
            (b"horse", b"stallion"),
            (b"shaman", b"horse"),
            (b"doge", b"coin"),
            (b"ether", b""),
            (b"dog", b"puppy"),
            (b"shaman", b""),
        ],
        "0x5991bb8c6514148a29db676a14ac506cd2cd5775ace63c30a4fe457715e9ac84",
    );
    assert_root(
        vec![
            (
                Vec::from_hex("04110d816c380812a427968ece99b1c963dfbce6")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("095e7baea6a6c7c4c2dfeb977efac326af552d87")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("0a517d755cebbf66312b30fff713666a9cb917e0")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("24dd378f51adc67a50e339e8031fe9bd4aafab36")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("293f982d000532a7861ab122bdc4bbfd26bf9030")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("2cf5732f017b0cf1b1f13a1478e10239716bf6b5")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("31c640b92c21a1f1465c91070b4b3b4d6854195f")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("37f998764813b136ddf5a754f34063fd03065e36")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("37fa399a749c121f8a15ce77e3d9f9bec8020d7a")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("4f36659fa632310b6ec438dea4085b522a2dd077")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("62c01474f089b07dae603491675dc5b5748f7049")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("729af7294be595a0efd7d891c9e51f89c07950c7")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("83e3e5a16d3b696a0314b30b2534804dd5e11197")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("8703df2417e0d7c59d063caa9583cb10a4d20532")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("8dffcd74e5b5923512916c6a64b502689cfa65e1")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("95a4d7cccb5204733874fa87285a176fe1e9e240")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("99b2fcba8120bedd048fe79f5262a6690ed38c39")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("a4202b8b8afd5354e3e40a219bdc17f6001bf2cf")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("a94f5374fce5edbc8e2a8697c15331677e6ebf0b")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("a9647f4a0a14042d91dc33c0328030a7157c93ae")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("aa6cffe5185732689c18f37a7f86170cb7304c2a")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("aae4a2e3c51c04606dcb3723456e58f3ed214f45")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("c37a43e940dfb5baf581a0b82b351d48305fc885")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("d2571607e241ecf590ed94b12d87c94babe36db6")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("f735071cbee190d76b704ce68384fc21e389fbe7")
                    .unwrap()
                    .as_slice(),
                b"something",
            ),
            (
                Vec::from_hex("04110d816c380812a427968ece99b1c963dfbce6")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("095e7baea6a6c7c4c2dfeb977efac326af552d87")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("0a517d755cebbf66312b30fff713666a9cb917e0")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("24dd378f51adc67a50e339e8031fe9bd4aafab36")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("293f982d000532a7861ab122bdc4bbfd26bf9030")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("2cf5732f017b0cf1b1f13a1478e10239716bf6b5")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("31c640b92c21a1f1465c91070b4b3b4d6854195f")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("37f998764813b136ddf5a754f34063fd03065e36")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("37fa399a749c121f8a15ce77e3d9f9bec8020d7a")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("4f36659fa632310b6ec438dea4085b522a2dd077")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("62c01474f089b07dae603491675dc5b5748f7049")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("729af7294be595a0efd7d891c9e51f89c07950c7")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("83e3e5a16d3b696a0314b30b2534804dd5e11197")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("8703df2417e0d7c59d063caa9583cb10a4d20532")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("8dffcd74e5b5923512916c6a64b502689cfa65e1")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("95a4d7cccb5204733874fa87285a176fe1e9e240")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("99b2fcba8120bedd048fe79f5262a6690ed38c39")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("a4202b8b8afd5354e3e40a219bdc17f6001bf2cf")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("a94f5374fce5edbc8e2a8697c15331677e6ebf0b")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("a9647f4a0a14042d91dc33c0328030a7157c93ae")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("aa6cffe5185732689c18f37a7f86170cb7304c2a")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("aae4a2e3c51c04606dcb3723456e58f3ed214f45")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("c37a43e940dfb5baf581a0b82b351d48305fc885")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("d2571607e241ecf590ed94b12d87c94babe36db6")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
            (
                Vec::from_hex("f735071cbee190d76b704ce68384fc21e389fbe7")
                    .unwrap()
                    .as_slice(),
                b"",
            ),
        ],
        "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    );
    assert_root(
        vec![
            (
                Vec::from_hex("0000000000000000000000000000000000000000000000000000000000000045")
                    .unwrap()
                    .as_slice(),
                Vec::from_hex("22b224a1420a802ab51d326e29fa98e34c4f24ea")
                    .unwrap()
                    .as_slice(),
            ),
            (
                Vec::from_hex("0000000000000000000000000000000000000000000000000000000000000046")
                    .unwrap()
                    .as_slice(),
                Vec::from_hex("67706c2076330000000000000000000000000000000000000000000000000000")
                    .unwrap()
                    .as_slice(),
            ),
            (
                Vec::from_hex("0000000000000000000000000000000000000000000000000000001234567890")
                    .unwrap()
                    .as_slice(),
                Vec::from_hex("697c7b8c961b56f675d570498424ac8de1a918f6")
                    .unwrap()
                    .as_slice(),
            ),
            (
                Vec::from_hex("000000000000000000000000697c7b8c961b56f675d570498424ac8de1a918f6")
                    .unwrap()
                    .as_slice(),
                Vec::from_hex("1234567890").unwrap().as_slice(),
            ),
            (
                Vec::from_hex("0000000000000000000000007ef9e639e2733cb34e4dfc576d4b23f72db776b2")
                    .unwrap()
                    .as_slice(),
                Vec::from_hex("4655474156000000000000000000000000000000000000000000000000000000")
                    .unwrap()
                    .as_slice(),
            ),
            (
                Vec::from_hex("000000000000000000000000ec4f34c97e43fbb2816cfd95e388353c7181dab1")
                    .unwrap()
                    .as_slice(),
                Vec::from_hex("4e616d6552656700000000000000000000000000000000000000000000000000")
                    .unwrap()
                    .as_slice(),
            ),
            (
                Vec::from_hex("4655474156000000000000000000000000000000000000000000000000000000")
                    .unwrap()
                    .as_slice(),
                Vec::from_hex("7ef9e639e2733cb34e4dfc576d4b23f72db776b2")
                    .unwrap()
                    .as_slice(),
            ),
            (
                Vec::from_hex("4e616d6552656700000000000000000000000000000000000000000000000000")
                    .unwrap()
                    .as_slice(),
                Vec::from_hex("ec4f34c97e43fbb2816cfd95e388353c7181dab1")
                    .unwrap()
                    .as_slice(),
            ),
            (
                Vec::from_hex("0000000000000000000000000000000000000000000000000000001234567890")
                    .unwrap()
                    .as_slice(),
                Vec::from_hex("").unwrap().as_slice(),
            ),
            (
                Vec::from_hex("000000000000000000000000697c7b8c961b56f675d570498424ac8de1a918f6")
                    .unwrap()
                    .as_slice(),
                Vec::from_hex("6f6f6f6820736f2067726561742c207265616c6c6c793f000000000000000000")
                    .unwrap()
                    .as_slice(),
            ),
            (
                Vec::from_hex("6f6f6f6820736f2067726561742c207265616c6c6c793f000000000000000000")
                    .unwrap()
                    .as_slice(),
                Vec::from_hex("697c7b8c961b56f675d570498424ac8de1a918f6")
                    .unwrap()
                    .as_slice(),
            ),
        ],
        "0x9f6221ebb8efe7cff60a716ecb886e67dd042014be444669f0159d8e68b42100",
    );
    assert_root(
        vec![
            (b"key1aa", b"0123456789012345678901234567890123456789xxx"),
            (
                b"key1",
                b"0123456789012345678901234567890123456789Very_Long",
            ),
            (b"key2bb", b"aval3"),
            (b"key2", b"short"),
            (b"key3cc", b"aval3"),
            (b"key3", b"1234567890123456789012345678901"),
        ],
        "0xcb65032e2f76c48b82b5c24b3db8f670ce73982869d38cd39a624f23d62a9e89",
    );
    assert_root(
        vec![(b"abc", b"123"), (b"abcd", b"abcd"), (b"abc", b"abc")],
        "0x7a320748f780ad9ad5b0837302075ce0eeba6c26e3d8562c67ccc0f1b273298a",
    );
}

// proof test ref:
// - https://github.com/ethereum/go-ethereum/blob/master/trie/proof_test.go
// - https://github.com/ethereum/py-trie/blob/master/tests/test_proof.py
#[test]
fn test_proof_basic() {
    let memdb = Arc::new(MemoryDB::new(true));
    let mut trie = InnerTrie::new(Arc::clone(&memdb));

    trie.insert(b"doe", b"reindeer").unwrap();
    trie.insert(b"dog", b"puppy").unwrap();
    trie.insert(b"dogglesworth", b"cat").unwrap();

    let root = trie.root_hash().unwrap();
    let r = format!("0x{}", hex::encode(trie.root_hash().unwrap()));
    assert_eq!(
        r.as_str(),
        "0x8aad789dff2f538bca5d8ea56e8abe10f4c7ba3a5dea95fea4cd6e7c3a1168d3"
    );

    // proof of key exists
    let proof = trie.get_proof(b"doe").unwrap();
    let expected = vec![
            "e5831646f6a0db6ae1fda66890f6693f36560d36b4dca68b4d838f17016b151efe1d4c95c453",
            "f83b8080808080ca20887265696e6465657280a037efd11993cb04a54048c25320e9f29c50a432d28afdf01598b2978ce1ca3068808080808080808080",
        ];
    assert_eq!(
        proof
            .clone()
            .into_iter()
            .map(hex::encode)
            .collect::<Vec<_>>(),
        expected
    );
    let value = trie.verify_proof(root, b"doe", proof).unwrap();
    assert_eq!(value, Some(b"reindeer".to_vec()));

    // proof of key not exist
    let proof = trie.get_proof(b"dogg").unwrap();
    let expected = vec![
            "e5831646f6a0db6ae1fda66890f6693f36560d36b4dca68b4d838f17016b151efe1d4c95c453",
            "f83b8080808080ca20887265696e6465657280a037efd11993cb04a54048c25320e9f29c50a432d28afdf01598b2978ce1ca3068808080808080808080",
            "e4808080808080ce89376c6573776f72746883636174808080808080808080857075707079",
        ];
    assert_eq!(
        proof
            .clone()
            .into_iter()
            .map(hex::encode)
            .collect::<Vec<_>>(),
        expected
    );
    let value = trie.verify_proof(root, b"dogg", proof).unwrap();
    assert_eq!(value, None);

    // empty proof
    let proof = vec![];
    let value = trie.verify_proof(root, b"doe", proof);
    assert!(value.is_err());

    // bad proof
    let proof = vec![b"aaa".to_vec(), b"ccc".to_vec()];
    let value = trie.verify_proof(root, b"doe", proof);
    assert!(value.is_err());
}

#[test]
fn test_proof_random() {
    let memdb = Arc::new(MemoryDB::new(true));
    let mut trie = InnerTrie::new(Arc::clone(&memdb));
    let mut rng = rand::thread_rng();
    let mut keys = vec![];
    for _ in 0..100 {
        let random_bytes: Vec<u8> = (0..rng.gen_range(2..30))
            .map(|_| rand::random::<u8>())
            .collect();
        trie.insert(&random_bytes, &random_bytes).unwrap();
        keys.push(random_bytes.clone());
    }
    for k in keys.clone().into_iter() {
        trie.insert(&k, &k).unwrap();
    }
    let root = trie.root_hash().unwrap();
    for k in keys.into_iter() {
        let proof = trie.get_proof(&k).unwrap();
        let value = trie.verify_proof(root, &k, proof).unwrap().unwrap();
        assert_eq!(value, k);
    }
}

#[test]
fn test_proof_empty_trie() {
    let memdb = Arc::new(MemoryDB::new(true));
    let mut trie = InnerTrie::new(Arc::clone(&memdb));
    trie.root_hash().unwrap();
    let proof = trie.get_proof(b"not-exist").unwrap();
    assert_eq!(proof.len(), 0);
}

#[test]
fn test_proof_one_element() {
    let memdb = Arc::new(MemoryDB::new(true));
    let mut trie = InnerTrie::new(Arc::clone(&memdb));
    trie.insert(b"k", b"v").unwrap();
    let root = trie.root_hash().unwrap();
    let proof = trie.get_proof(b"k").unwrap();
    assert_eq!(proof.len(), 1);
    let value = trie.verify_proof(root, b"k", proof.clone()).unwrap();
    assert_eq!(value, Some(b"v".to_vec()));

    // remove key does not affect the verify process
    trie.remove(b"k").unwrap();
    let _root = trie.root_hash().unwrap();
    let value = trie.verify_proof(root, b"k", proof).unwrap();
    assert_eq!(value, Some(b"v".to_vec()));
}
