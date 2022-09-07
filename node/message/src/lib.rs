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
    #[test]
    fn it_works2() {
        //hold on as Daniel refactors
        //make sure node can work w messages

        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut messageHandler = MessageHandler::new(
            tx, rx);
        let mut commandHandler = CommandHandler::new();
        node.start();
        
    }
}