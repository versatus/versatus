#[cfg(test)]
mod tests {
    //extract command handler, hook it up elsewhere and test
    //test msg handler in isolation
    use crate::node::Node;
    use crate::handler::{Handler, CommandHandler, MessageHandler};
    use commands::command::Command;
    
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    //document describing how to build the docker image (not a docker container)
    //how to use diff command line flags

    //run cargo run
    //cargo run -- -help
    //in clie define command line flags
    //we are using clap framework (a crate) -- declarative macro variant
    //submit pR modifying readme with tutorial and decription on how to run nodes
        //can ask for help to see flags

    /*
    You might need 2 threads one running a socket on one port and one on another. 
    To test the senders and receivers you need at least one thread to send from and 
    the receiver can be in the main thread (general test scope) */

    #[test]
    fn send_comand() {

        let (mining_sender, mining_receiver) = mpsc::unbounded();
        let (blockchain_sender, blockchain_receiver) = mpsc::unbounded();
        let (gossip_sender, gossip_receiver) = mpsc::unbounded();
        let (swarm_sender, swarm_receiver) = mpsc::unbounded();
        let (state_sender, state_receiver) = mpsc::unbounded();
        let (gossip_tx_sender, gossip_tx_receiver) = channel();
        let (ctrl_sender, ctrl_receiver) = channel();



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