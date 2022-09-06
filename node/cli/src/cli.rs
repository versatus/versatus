/// Everything on this crate is tentative and meant to be a stepping stone into the finalized
/// version soon.
use clap::Parser;
use std::str::FromStr;
use thiserror::Error;

type Result<T> = std::result::Result<T, CliError>;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid node type {0} provided")]
    InvalidNodeType(String),
}

#[derive(Debug, Clone)]
pub enum NodeType {
    /// A Node that can archive, validate and mine tokens
    Full,
    /// Same as `NodeType::Full` but without archiving capabilities
    Light,
    /// Archives all transactions processed in the blockchain
    Archive,
    /// Mining node
    Miner,
    Bootstrap,
    Validator,
}

impl FromStr for NodeType {
    type Err = CliError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // TODO: define node types thoroughly
        match s {
            "full" => Ok(NodeType::Full),
            "light" => Ok(NodeType::Light),
            _ => Err(CliError::InvalidNodeType(s.into())),
        }
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// node_type defines the type of node created by this program
    #[clap(short, long, value_parser, default_value = "full")]
    pub node_type: String,
}

pub fn parse() -> Result<Cli> {
    Ok(Cli::parse())
}
