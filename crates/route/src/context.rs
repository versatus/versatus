
use std::{
    sync::Arc,
};

use clap::Parser;

use crate::lrnodepool::LeftRightNodePoolDB;

/// Dispersed routes discovery cluster app. Simplified to enable easier cluster fault detection.
#[derive(Parser, Debug, Clone)]
#[command(version)]
pub struct AppParams {
    /// Full bind address in form of /ip4|ip6/<address IP>/<proto>/<port>
    #[arg(short, long, default_value_t = String::from("/ip4/0.0.0.0/tcp/0"))]
    pub full_bind_address: String,
}

pub struct AppContext {
    pub node_routes_db: LeftRightNodePoolDB,
    pub args: Arc<AppParams>,
    // pub log_file: Arc<File>,
}

impl AppContext {
    pub fn new() -> Self {

        let args = AppParams::parse();

        AppContext {
            node_routes_db: LeftRightNodePoolDB::new(),
            args: Arc::new(args),
        }
    }
}
