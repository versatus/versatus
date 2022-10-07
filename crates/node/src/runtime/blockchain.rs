use block::invalid::InvalidBlockErrorReason;
use blockchain::blockchain::Blockchain;
use commands::command::Command;
use telemetry::{error, info};
use tokio::sync::mpsc::error::TryRecvError;

use crate::{
    command_router::{CommandPublisher, DirectedCommand},
    result::Result,
    NodeError, RuntimeModule, RuntimeModuleState,
};

pub struct BlockchainModule {
    blockchain: Blockchain,
    cmd_publisher: CommandPublisher,
    runnig_status: RuntimeModuleState,
}

impl RuntimeModule for BlockchainModule {
    fn name(&self) -> String {
        String::from("Blockcchain module")
    }

    fn status(&self) -> crate::RuntimeModuleState {
        self.runnig_status.clone()
    }

    fn start(
        &mut self,
        control_rx: &mut tokio::sync::mpsc::UnboundedReceiver<commands::command::Command>,
    ) -> Result<()> {
        self.runnig_status = RuntimeModuleState::Running;
        // TODO: rethink this loop
        loop {
            match control_rx.try_recv() {
                Ok(cmd) if cmd == Command::Stop => {
                    telemetry::info!("Received stop signal");
                    self.runnig_status = RuntimeModuleState::Terminating;

                    break;
                },
                Ok(cmd) => {
                    if let Err(err) = self.handle_command(cmd) {
                        error!("{:?}", err);
                    }
                },
                Err(err) if err == TryRecvError::Disconnected => {
                    telemetry::warn!("Failed to process stop signal. Reason: {0}", err);
                    telemetry::warn!("{} shutting down", self.name());
                    break;
                },
                _ => {},
            }
        }

        self.runnig_status = RuntimeModuleState::Stopped;
        Ok(())
    }
}

impl BlockchainModule {
    pub fn new(cmd_publisher: CommandPublisher) -> Self {
        Self {
            blockchain: Blockchain::new("dummy.db"),
            cmd_publisher,
            runnig_status: RuntimeModuleState::Stopped,
        }
    }

    fn publish(&mut self, cmd: DirectedCommand) -> Result<()> {
        if let Err(err) = self.cmd_publisher.send(cmd) {
            return Err(NodeError::Other(err.to_string()));
        }

        Ok(())
    }

    fn handle_command(&mut self, cmd: commands::command::Command) -> Result<()> {
        // TODO: re-add the apropriate handlers
        match cmd {
            Command::PendingBlock(block_bytes, sender_id) => {},
            Command::GetStateComponents(requestor, components_bytes, sender_id) => {},
            Command::StoreStateComponents(component_bytes, component_type) => {},
            Command::ProcessBacklog => {},
            Command::StateUpdateCompleted(network_state) => {},
            Command::ClaimAbandoned(pubkey, claim_bytes) => {},
            Command::SlashClaims(bad_validators) => {},
            Command::NonceUp => {},
            Command::GetHeight => {},
            _ => {},
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::command_router::DirectedCommand;

    use super::*;

    #[test]
    fn should_handle_stop_command() {
        let (command_tx, mut command_rx) =
            tokio::sync::mpsc::unbounded_channel::<DirectedCommand>();

        let mut blockchain_module = BlockchainModule::new(command_tx.clone());

        assert_eq!(blockchain_module.status(), RuntimeModuleState::Stopped);
        blockchain_module.handle_command(Command::Stop).unwrap();
        assert_eq!(blockchain_module.status(), RuntimeModuleState::Stopped);
    }

    #[tokio::test]
    #[ignore = "not implemented yet"]
    async fn should_stop_when_issued_stop_command() {
        todo!()
        //     // let (command_tx, mut command_rx) =
        //     //     tokio::sync::mpsc::unbounded_channel::<DirectedCommand>();
        //     // let mut router = CommandRouter::new();
        //     //
        //     // let handle = tokio::spawn(async move {
        //     //     // router.start(&mut command_rx).await.unwrap();
        //     //     router.start(&mut command_rx).await.unwrap();
        //     // });
        //     //
        //     // command_tx
        //     //     .send((CommandRoute::Router, Command::Stop))
        //     //     .unwrap();
        //     //
        //     // handle.await.unwrap();
    }
}
