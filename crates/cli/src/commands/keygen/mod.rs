use crate::result::{CliError, Result};
use clap::Parser;
use std::path::PathBuf;
use telemetry::{info, warn};
use vrrb_core::keypair::{read_keypair_file, write_keypair_file, Keypair};

#[derive(Debug, Parser)]
pub struct KeygenCmd {
    /// Overwrite the existing keypair if it exists.
    #[clap(long)]
    force: bool,
}

pub fn exec(args: KeygenCmd) -> Result<()> {
    println!(
        "PublicKey: {}",
        keygen(args.force)?.miner_public_key_owned()
    );

    Ok(())
}

/// Attempts to read a keypair from file, and generates a new keypair
/// if one does not exist at the expected path.
pub fn keygen(overwrite: bool) -> Result<Keypair> {
    let data_dir = vrrb_core::storage_utils::get_node_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;
    let keypair_file_path = PathBuf::from(&data_dir).join("keypair");
    match read_keypair_file(&keypair_file_path) {
        Ok(keypair) => {
            if overwrite {
                info!("Found stale keypair file, overwriting with new keypair");
                write_new_keypair(&keypair_file_path)
            } else {
                info!("Found existing keypair");
                Ok(keypair)
            }
        },
        Err(err) => {
            warn!("Failed to read keypair file: {err}");
            info!("Generating new keypair");
            write_new_keypair(&keypair_file_path)
        },
    }
}

fn write_new_keypair(outfile: &PathBuf) -> Result<Keypair> {
    let keypair = Keypair::random();
    write_keypair_file(&keypair, outfile)
        .map_err(|err| CliError::Other(format!("failed to write keypair file: {err}")))?;
    info!("Successfully wrote new keypair to file");

    Ok(keypair)
}
