use crate::web3_pkg::{
    Web3ContentId, Web3ObjectType, Web3Package, Web3PackageArchitecture, Web3PackageBuilder,
    Web3PackageObject, Web3PackageObjectBuilder, Web3PackageType,
};
use crate::web3_store::Web3Store;
use std::str;
use tokio;

// The majority of the tests in this module are marked as #[ignore]. This is because in order for
// them to run, they require a separate IPFS service (such as Kubo) to be running. There is a task
// open to also embed an IPFS implementation (ipfs-embed) into the blob storage stack. Once this
// works, these tests will be able to be executed without the external dependency and can then be
// re-enabled. They're here in the meantime to allow testing inside my workspace.

/// This test just shows that we can use both builder interfaces and have them return Ok. A basic
/// smoke test if anything.
#[test]
#[ignore]
fn builder_test() {
    let obj = Web3PackageObjectBuilder::default()
        .object_arch(Web3PackageArchitecture::Wasm32Wasi)
        .object_path("wasm_test-opt.wasm".to_string())
        .object_cid(Web3ContentId {
            cid: "QmYSjeNygNwKP4E2vhZUs888UtdFXdpdZfkq7L4rH9nFgn".to_string(),
        })
        .build()
        .unwrap();
    let pkg = Web3PackageBuilder::default()
        .pkg_version(4)
        .pkg_name("Versatus Smart Contract".to_string())
        .pkg_author("Versatus Labs".to_string())
        .pkg_type(Web3PackageType::SmartContract)
        .pkg_objects(vec![obj])
        .pkg_replaces(vec![Web3ContentId {
            cid: "bafyreialcti7pn4eqgrdkr3aug45mhcqm65htuwkmtdw5pwth73a5o7piu".to_string(),
        }])
        .build();
    println!("pkg: {:?}", pkg);
    assert!(pkg.is_ok());
}

/// Tests parsing of package JSON
#[test]
fn parse_test() {
    let bytes = std::fs::read("test_data/pkg.json").unwrap();
    let json = str::from_utf8(&bytes).unwrap();
    let pkg: Web3Package = serde_json::from_str(&json).unwrap();
    eprintln!("Object: {:?}", pkg);
    assert_eq!(pkg.pkg_version, 5);
}

#[tokio::test]
#[ignore]
async fn stats_test() {
    let store = Web3Store::local().unwrap();
    let stats = store.stats().await.unwrap();
    eprintln!("Stats: {:?}", stats);
}

/// This pulls a DAG package from IPFS given a hard-coded CID, and then the contained objects.
#[tokio::test]
#[ignore]
async fn read_test() {
    let store = Web3Store::local().unwrap();
    const CID: &str = "bafyreigqcj4jbrfpwwdcdllqc5mc6xcqjoyd4mg4dklj7gtaxdp4zvn2ae";
    let obj = store.read_dag(CID).await.unwrap();
    eprintln!("Read manifest as DAG object from: {}", CID);
    let json = str::from_utf8(&obj).unwrap();
    //eprintln!("JSON: {}", json);
    std::fs::write("out/package-manifest.json", &json).unwrap();
    eprintln!("Wrote package manifest to out/package-manifest.json");
    let pkg: Web3Package = serde_json::from_str(&json).unwrap();
    for obj in &pkg.pkg_objects {
        let blob = store.read_object(&obj.object_cid.cid).await.unwrap();
        let filename = format!("out/{}", obj.object_cid.cid);
        std::fs::write(filename, blob).unwrap();
        eprintln!("Linked object written to out/{}", obj.object_cid.cid);
    }

    //eprintln!("Object returned: {:?}", pkg);
}

/// This creates a package and writes it
#[tokio::test]
#[ignore]
async fn write_test() {
    // create an instance of the store
    let store = Web3Store::local().unwrap();
    let mut objects: Vec<Web3PackageObject> = vec![];

    // Add the first object
    let cid = store
        .write_object(std::fs::read("test_data/wasm_test-opt.wasm").unwrap())
        .await
        .unwrap();
    eprintln!("Added WASM file as CID: {}", cid);
    // Wrap the object to include in the package.
    let obj = Web3PackageObjectBuilder::default()
        .object_arch(Web3PackageArchitecture::Wasm32Wasi)
        .object_path("wasm_test-opt.wasm".to_string())
        .object_cid(Web3ContentId { cid: cid })
        .object_type(Web3ObjectType::Executable)
        .build()
        .unwrap();
    objects.push(obj);

    // Add the second object
    let cid = store
        .write_object(std::fs::read("test_data/README.md").unwrap())
        .await
        .unwrap();
    eprintln!("Added README.md as CID: {}", cid);
    // Wrap the object to include in the package.
    let obj = Web3PackageObjectBuilder::default()
        .object_arch(Web3PackageArchitecture::None)
        .object_path("README.md".to_string())
        .object_cid(Web3ContentId { cid: cid })
        .object_type(Web3ObjectType::Document)
        .build()
        .unwrap();
    objects.push(obj);

    // Create the package containing the above object(s)
    let pkg = Web3PackageBuilder::default()
        .pkg_version(5)
        .pkg_name("Versatus Smart Contract".to_string())
        .pkg_author("Versatus Labs".to_string())
        .pkg_type(Web3PackageType::SmartContract)
        .pkg_objects(objects)
        .pkg_replaces(vec![Web3ContentId {
            cid: "bafyreialcti7pn4eqgrdkr3aug45mhcqm65htuwkmtdw5pwth73a5o7piu".to_string(),
        }])
        .build()
        .unwrap();
    let json = serde_json::to_string(&pkg).unwrap();

    println!("JSON: {}", json);

    // write the root of the DAG
    let cid = store.write_dag(json.into()).await.unwrap();
    eprintln!("DAG write of root (package) returned CID: {}", cid);
}
