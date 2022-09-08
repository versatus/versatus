pub mod handler;

#[cfg(test)]
mod tests {
    use crate::handler::{Handler, CommandHandler, MessageHandler};
    use commands::command::Command;
    use tokio::sync::mpsc;
    use log::info;
    use std::{sync::mpsc::Sender, thread};
    
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    /*
    You might need 2 threads one running a socket on one port and one on another. 
    To test the senders and receivers you need at least one thread to send from and 
    the receiver can be in the main thread (general test scope) */

    #[tokio::test]
    async fn send_comand() {

        let (mining_sender, mining_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut mining_handler = MessageHandler::new(mining_sender, mining_receiver);
            
        let (blockchain_sender, blockchain_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let blockchain_handler = MessageHandler::new(blockchain_sender, blockchain_receiver);

        let (gossip_sender, gossip_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let gossip_handler = MessageHandler::new(gossip_sender, gossip_receiver);

        let (swarm_sender, swarm_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let swarm_handler = MessageHandler::new(swarm_sender, swarm_receiver);

        let (state_sender, mut state_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let state_handler = MessageHandler::new(state_sender, state_receiver);

        let (gossip_tx_sender, gossip_tx_receiver) = 
        std::sync::mpsc::channel();

        let (ctrl_sender, mut ctrl_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let ctrl_handler = MessageHandler::new(ctrl_sender, ctrl_receiver);

        let mut commandHandler = CommandHandler::new(
            mining_handler.sender,
            blockchain_handler.sender,
            gossip_handler.sender,
            swarm_handler.sender,
            state_handler.sender,
            gossip_tx_sender,
            ctrl_handler.receiver 
        );

        let mut claim = Vec::new();
        claim.push(0);
        claim.push(1);
        claim.push(2);

        let process_claim = Command::ProcessClaim(claim);

        ctrl_handler.sender.send(process_claim).unwrap();

        let cmd = commandHandler.receiver.recv().await.unwrap();

        commandHandler.handle_command(cmd);

        let cmd = mining_handler.receiver.recv().await.unwrap();

        let mut passing = false;

        if let Command::ProcessClaim(claim) = cmd {
                passing = true;
        }
          
        assert!(passing);  
    }

    #[tokio::test]
    async fn send_comand_over_thread() {

        let (mining_sender, mining_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut mining_handler = MessageHandler::new(mining_sender, mining_receiver);
            
        let (blockchain_sender, blockchain_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let blockchain_handler = MessageHandler::new(blockchain_sender, blockchain_receiver);

        let (gossip_sender, gossip_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let gossip_handler = MessageHandler::new(gossip_sender, gossip_receiver);

        let (swarm_sender, swarm_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let swarm_handler = MessageHandler::new(swarm_sender, swarm_receiver);

        let (state_sender, mut state_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let state_handler = MessageHandler::new(state_sender, state_receiver);

        let (gossip_tx_sender, gossip_tx_receiver) = 
        std::sync::mpsc::channel();

        let (ctrl_sender, mut ctrl_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let ctrl_handler = MessageHandler::new(ctrl_sender, ctrl_receiver);

        let mut commandHandler = CommandHandler::new(
            mining_handler.sender,
            blockchain_handler.sender,
            gossip_handler.sender,
            swarm_handler.sender,
            state_handler.sender,
            gossip_tx_sender,
            ctrl_handler.receiver 
        );

        thread::spawn(move || {
            let mut claim = Vec::new();
            claim.push(0);
            claim.push(1);
            claim.push(2);

            let process_claim = Command::ProcessClaim(claim);

            ctrl_handler.sender.send(process_claim).unwrap();
        });
            
        let cmd = commandHandler.receiver.recv().await.unwrap();

        commandHandler.handle_command(cmd);

        let cmd = mining_handler.receiver.recv().await.unwrap();

        let mut passing = false;

        if let Command::ProcessClaim(claim) = cmd {
                passing = true;
        }
        
        assert!(passing);  
    }
}