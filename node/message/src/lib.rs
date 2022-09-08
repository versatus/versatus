#[cfg(test)]
mod tests {
    use crate::msghandler::{Handler, CommandHandler, MessageHandler};
    use commands::command::Command;
    use tokio::sync::mpsc;
    use messages::message::Message;
    
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    /*
    You might need 2 threads one running a socket on one port and one on another. 
    To test the senders and receivers you need at least one thread to send from and 
    the receiver can be in the main thread (general test scope) */

    #[test]
    fn send_comand() {

        let (m_sender, m_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let (mining_sender, mining_receiver) = MessageHandler::new(m_sender, m_receiver);
            
        let (b_sender, b_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let (blockchain_sender, blockchain_receiver) = MessageHandler::new(b_sender, b_receiver);

        let (g_sender, g_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let (gossip_sender, gossip_receiver) = MessageHandler::new(g_sender, g_receiver);

        let (sw_sender, sw_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let (swarm_sender, swarm_receiver) = MessageHandler::new(sw_sender, sw_receiver);


        let (s_sender, s_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let (state_sender, state_receiver) = MessageHandler::new(s_sender, s_receiver);

        let (g_tx_sender, g_tx_receiver) = 
        tokio::sync::mpsc::channel::<Message>(100);
        let (gossip_tx_sender, gossip_tx_receiver) = MessageHandler::new(g_tx_sender, g_tx_receiver);

        let (c_sender, c_receiver) = 
        tokio::sync::mpsc::unbounded_channel::<Command>();
        let (ctrl_sender, ctrl_receiver) = MessageHandler::new(c_sender, c_receiver);


        let commandHandler = CommandHandler::new(
            mining_sender,
            blockchain_sender,
            gossip_sender,
            swarm_sender,
            state_sender,
            gossip_tx_sender,
            ctrl_receiver 
        );

        let getState = Command::GetState;
        ctrl_sender.send(getState);

        commandHandler.handle_command(commandHandler.ctrl_receiver.recv());

        assert!(state_receiver.recv().unwrap() == getState);        
    }
}