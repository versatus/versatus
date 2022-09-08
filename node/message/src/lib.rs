#[cfg(test)]
mod tests {
    //extract command handler, hook it up elsewhere and test
    //test msg handler in isolation
    use crate::node::Node;
    use crate::handler::{Handler, CommandHandler, MessageHandler};
    
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
    fn it_works2() {
        

        //let (mining_sender: Sender<{unknown}>, mining_receiver: Receiver<{unknown}>) = unbounded();


        /* 
        let (tx, mut rx) = mpsc::unbounded_channel();
        let messageHandler = MessageHandler::new(tx, rx);

        thread::spawn(move || {
            let mut messageHandler = MessageHandler::new(
                tx, rx);
            let command = Command::new("ls");
            tx.send(val).unwrap();
        });
    
        let received = rx.recv().unwrap();
        assert!(received == "hi");
        */
        //let mut messageHandler = MessageHandler::new(tx, rx);

        //let mut commandHandler = CommandHandler::new();

        //test with two threads and with one thread
        
    }
}