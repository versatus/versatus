use std::process::Command;

use telemetry::info;
use tokio::task::JoinHandle;
use vrrb_config::NodeConfig;

use crate::result::{NodeError, Result};

pub(crate) async fn setup_node_gui(config: &NodeConfig) -> Result<Option<JoinHandle<Result<()>>>> {
    if config.gui {
        info!("Configuring Node {}", &config.id);
        info!("Ensuring environment has required dependencies");

        match Command::new("npm").args(["version"]).status() {
            Ok(_) => info!("NodeJS is installed"),
            Err(e) => {
                return Err(NodeError::Other(format!("NodeJS is not installed: {e}")));
            },
        }

        info!("Ensuring yarn is installed");
        match Command::new("yarn").args(["--version"]).status() {
            Ok(_) => info!("Yarn is installed"),
            Err(e) => {
                let install_yarn = Command::new("npm")
                    .args(["install", "-g", "yarn"])
                    .current_dir("infra/gui")
                    .output();

                match install_yarn {
                    Ok(_) => (),
                    Err(_) => {
                        return Err(NodeError::Other(format!("Failed to install yarn: {e}")));
                    },
                }
            },
        }

        info!("Installing dependencies");
        match Command::new("yarn")
            .args(["install"])
            .current_dir("infra/gui")
            .status()
        {
            Ok(_) => info!("Dependencies installed successfully"),
            Err(e) => {
                return Err(NodeError::Other(format!(
                    "Failed to install dependencies: {e}"
                )));
            },
        }

        info!("Spawning UI");

        let node_gui_handle = tokio::spawn(async move {
            if let Err(err) = Command::new("yarn")
                .args(["dev"])
                .current_dir("infra/gui")
                .spawn()
            {
                telemetry::error!("Failed to spawn UI: {}", err);
            }

            Ok(())
        });

        info!("Finished spawning UI");
        Ok(Some(node_gui_handle))
    } else {
        info!("GUI not enabled");
        Ok(None)
    }
}
