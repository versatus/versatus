pub mod node;
pub mod handler;


/*let some_task = listener.start();
send_the_msg();
let some_task_result = some_task.await;
assert_eq!(blahblah, blahblah);
*/

#[cfg(test)]
mod tests {
    //extract command handler, hook it up elsewhere and test
    //test msg handler in isolation
    use crate::node::Node;
    use crate::handler::{Handler, CommandHandler, MessageHandler};
    use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
    
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    /*
    #[test]
    fn it_works2() {
        //hold on as Daniel refactors

        //let (tx, mut rx) = mpsc::unbounded_channel();
        //let mut messageHandler = MessageHandler::new(
        //    tx, rx);
        //let mut commandHandler = CommandHandler::new();
        //let mut node = Node::new();
        //node.start();
        
    }
    */
}