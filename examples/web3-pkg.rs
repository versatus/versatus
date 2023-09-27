use anyhow::{anyhow, Result};
use clap::Parser;
use serde_derive::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::path::Path;
use std::str;

use web3_pkg::web3_pkg::{
    Web3ContentId, Web3ObjectType, Web3Package, Web3PackageArchitecture, Web3PackageBuilder,
    Web3PackageObject, Web3PackageObjectBuilder, Web3PackageType,
};
use web3_pkg::web3_store::Web3Store;

#[derive(Parser)]
#[clap(author, version, about)]
struct PackageCli {
    #[clap(subcommand)]
    pub cmd: PackageCommands,
}

#[derive(Parser)]
pub enum PackageCommands {
    /// Create a new package definition file.
    Init(InitOpts),
    /// Add a link to an object in a package definition file.
    AddObject(AddObjectOpts),
    /// Build and publish a web3 package from a package definition.
    Build(BuildOpts),
    /// Retrieve a web3 package and its objects to a local directory.
    Retrieve(RetrieveOpts),
}

#[derive(Parser)]
pub struct InitOpts {
    /// A JSON file to build to describe and then publish a package.
    #[clap(
        short,
        long,
        value_parser,
        value_name = "FILENAME",
        default_value = "./web3-pkg.json"
    )]
    config: String,
    /// Version number for this package
    #[clap(short, long, value_parser, value_name = "VERSION")]
    version: u32,
    /// Package name for this package
    #[clap(short, long, value_parser, value_name = "NAME")]
    name: String,
    /// Package author
    #[clap(short, long, value_parser, value_name = "AUTHOR")]
    author: String,
    /// Package type
    #[clap(short, long, value_parser, value_name = "TYPE")]
    r#type: Web3PackageType,
    #[clap(short = 'o', long, value_parser, value_name = "KEY=VALUE")]
    annotations: Vec<String>,
}

#[derive(Parser)]
pub struct AddObjectOpts {
    /// A JSON file to build to describe and then publish a package.
    #[clap(
        short,
        long,
        value_parser,
        value_name = "FILENAME",
        default_value = "./web3-pkg.json"
    )]
    config: String,
    /// The architecture the object is targetted to.
    #[clap(short, long, value_parser, value_name = "ARCH")]
    architecture: Web3PackageArchitecture,
    /// The type of object.
    #[clap(short, long, value_parser, value_name = "TYPE")]
    r#type: Web3ObjectType,
    /// Path to the object on the local filesystem
    #[clap(short, long, value_parser, value_name = "PATH")]
    path: String,
    /// Annotations to store with the object.
    #[clap(short = 'o', long, value_parser, value_name = "KEY=VALUE")]
    annotations: Vec<String>,
}

#[derive(Parser)]
pub struct BuildOpts {
    /// A JSON file to build to describe and then publish a package.
    #[clap(
        short,
        long,
        value_parser,
        value_name = "FILENAME",
        default_value = "./web3-pkg.json"
    )]
    config: String,
}

#[derive(Parser)]
pub struct RetrieveOpts {
    #[clap(short, long, value_parser, value_name = "CID")]
    cid: String,
    #[clap(short, long, value_parser, value_name = "OUTDIR")]
    outdir: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PackageMetadata {
    version: u32,
    name: String,
    author: String,
    r#type: Web3PackageType,
    annotations: Vec<(String, String)>,
    objects: Vec<PackageObject>,
}

impl PackageMetadata {
    pub fn from_file(filename: &str) -> Result<Self> {
        let bytes = std::fs::read(filename)?;
        let json = str::from_utf8(&bytes)?;
        let pkg: PackageMetadata = serde_json::from_str(json)?;
        Ok(pkg)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PackageObject {
    architecture: Web3PackageArchitecture,
    path: String,
    content_id: Option<String>,
    r#type: Web3ObjectType,
    annotations: Vec<(String, String)>,
}

async fn package_init(opts: &InitOpts) -> Result<()> {
    // Use opts to initialise a new JSON file
    let pkg = PackageMetadata {
        version: opts.version,
        name: opts.name.clone(),
        author: opts.author.clone(),
        r#type: opts.r#type.clone(),
        annotations: parse_annotations(&opts.annotations).await?,
        objects: vec![],
    };
    let json = serde_json::to_string(&pkg)?;
    std::fs::write(&opts.config, json)?;
    Ok(())
}

async fn parse_annotations(annotations: &Vec<String>) -> Result<Vec<(String, String)>> {
    let mut ret = vec![];
    for a_str in annotations.iter() {
        let mut split = a_str.split('=');
        let (key, value) = (
            split
                .next()
                .ok_or(anyhow!(
                    "Unable to parse annotation. Use 'key=value' syntax."
                ))?
                .to_string(),
            split
                .next()
                .ok_or(anyhow!(
                    "Unable to parse annotation. Use 'key=value' syntax."
                ))?
                .to_string(),
        );
        ret.push((key, value));
    }
    Ok(ret)
}

async fn package_addobject(opts: &AddObjectOpts) -> Result<()> {
    let obj = PackageObject {
        architecture: opts.architecture.clone(),
        content_id: None,
        path: opts.path.clone(),
        r#type: opts.r#type.clone(),
        annotations: parse_annotations(&opts.annotations).await?,
    };

    let mut pkg = PackageMetadata::from_file(&opts.config)?;

    // push object
    pkg.objects.push(obj);

    // serialise to JSON
    let json = serde_json::to_string(&pkg)?;
    std::fs::write(&opts.config, json)?;

    Ok(())
}

async fn package_build(opts: &BuildOpts) -> Result<()> {
    // Use package description to build and publish a package
    let pkg = PackageMetadata::from_file(&opts.config)?;

    let store = Web3Store::local()?;
    let mut objects: Vec<Web3PackageObject> = vec![];

    // loop through each object and add them to the blob store, saving the CID
    for blob in pkg.objects.iter() {
        // Write the blob
        let cid = store.write_object(std::fs::read(&blob.path)?).await?;

        // Keep the filename portion of the path, but without parent directories. The filename
        // could act as a user-readable label, but has no real technical significance, and the full
        // path including directories could potentially be sensitive and at very least irrelevant.
        let path = Path::new(&blob.path)
            .file_name()
            .unwrap_or(OsStr::new(""))
            .to_str()
            .unwrap_or_default();

        // store the blob's metadata ready to write to the DAG
        let obj = Web3PackageObjectBuilder::default()
            .object_arch(blob.architecture.to_owned())
            .object_path(path.to_string().to_owned())
            .object_cid(Web3ContentId { cid })
            .object_type(blob.r#type.to_owned())
            .build()?;
        objects.push(obj);
    }

    // TODO: add pkg_replaces() if present.
    let pkg_meta = Web3PackageBuilder::default()
        .pkg_version(pkg.version)
        .pkg_name(pkg.name)
        .pkg_author(pkg.author)
        .pkg_type(pkg.r#type)
        .pkg_objects(objects)
        .pkg_replaces(vec![])
        .build()?;
    let json = serde_json::to_string(&pkg_meta)?;

    // Write a Web3Package DAG to blob store and return the CID
    let cid = store.write_dag(json.into()).await?;

    // Show the user the CID of the package for use elsewhere.
    println!("Content ID for Web3 Package is {}", cid);
    Ok(())
}

async fn package_retrieve(opts: &RetrieveOpts) -> Result<()> {
    // Use opts to initialise a new JSON file
    let store = Web3Store::local()?;

    // read the DAG root
    let obj = store.read_dag(&opts.cid).await?;
    let json = str::from_utf8(&obj)?;
    // store the DAG root as the package manifest
    std::fs::write(format!("{}/package-manifest.json", opts.outdir), json)?;

    let pkg: Web3Package = serde_json::from_str(json)?;

    // Retrive each of the child objects by CID
    for obj in &pkg.pkg_objects {
        let blob = store.read_object(&obj.object_cid.cid).await?;
        std::fs::write(format!("{}/{}", opts.outdir, &obj.object_cid.cid), blob)?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // parse command line options
    let cli = PackageCli::parse();

    // call subcommand
    match &cli.cmd {
        PackageCommands::Init(opts) => package_init(opts).await?,
        PackageCommands::AddObject(opts) => package_addobject(opts).await?,
        PackageCommands::Build(opts) => package_build(opts).await?,
        PackageCommands::Retrieve(opts) => package_retrieve(opts).await?,
    }

    Ok(())
}
