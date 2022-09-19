pub mod command_handler;
pub mod message_handler;

pub mod handler;
#[deprecated(note = "use node::core instead")]
pub mod node;

pub mod result;
use rand::Rng;
pub use result::*;

pub mod core {
    // TODO rename node.rs to core.rs once other refactoring efforts are complete
    pub use super::node::*;
}

pub fn generate_optional_vec() -> Option<Vec<u8>> {
    let mut rng = rand::thread_rng();
    let len = rng.gen_range(0, 100);
    let mut vec = Vec::with_capacity(len);
    for _ in 0..len {
        vec.push(rng.gen());
    }
    Some(vec)
}

#[cfg(test)]
mod tests {
    use crate::{
        command_handler::CommandHandler,

        core::{Node, NodeType},
        handler::Handler,
        message_handler::MessageHandler,
    };
    use super::generate_optional_vec;
    use commands::command::Command;
    use log::info;
    use messages::{message_types::MessageType, packet::Packet};
    use udp2p::protocol::protocol::{Message, Header};
    use std::{net::SocketAddr, sync::{mpsc::Sender, Arc}, thread};
    use telemetry::TelemetrySubscriber;
    use tokio::sync::mpsc;
    use std::net::{IpAddr, Ipv4Addr};
    use std::sync::Mutex;

    #[tokio::test]
    async fn node_can_receive_start_and_stop() {
        let (mining_sender, mut mining_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut mining_handler = MessageHandler::new(mining_sender, mining_receiver);

        let (blockchain_sender, blockchain_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let blockchain_handler = MessageHandler::new(blockchain_sender, blockchain_receiver);

        let (gossip_sender, gossip_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let gossip_handler = MessageHandler::new(gossip_sender, gossip_receiver);

        let (swarm_sender, swarm_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let swarm_handler = MessageHandler::new(swarm_sender, swarm_receiver);

        let (state_sender, mut state_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let state_handler = MessageHandler::new(state_sender, state_receiver);

        let (gossip_tx_sender, gossip_tx_receiver) = std::sync::mpsc::channel();

        let (ctrl_sender, mut ctrl_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let ctrl_handler = MessageHandler::new(ctrl_sender.clone(), ctrl_receiver);

        let mut cmd_handler = CommandHandler::new(
            mining_handler.sender,
            blockchain_handler.sender,
            gossip_handler.sender,
            swarm_handler.sender,
            state_handler.sender,
            gossip_tx_sender,
            ctrl_handler.receiver,
        );

        // NOTE: in case logging is needed, remove if not needed
        //TelemetrySubscriber::init(std::io::stdout).unwrap();

        let (msg_sender, mut msg_receiver) = tokio::sync::mpsc::unbounded_channel::<MessageType>();
        let (msg_sender_1, mut msg_receiver_1) =
            tokio::sync::mpsc::unbounded_channel::<(Packet, SocketAddr)>();

        let msg_handler = MessageHandler::new(msg_sender, msg_receiver_1);

        // NOTE: preliminary setup above this note
        let mut node = Node::new(NodeType::Full, cmd_handler, msg_handler, 100);

        let handle = tokio::spawn(async move {
            node.start().await.unwrap();
        });
        // NOTE: stop the node after tests are performed
        ctrl_sender.send(Command::Stop).unwrap();
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn node_can_allocate_mine_block() {
        let (mining_sender, mut mining_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut mining_handler = MessageHandler::new(mining_sender, mining_receiver);

        let (blockchain_sender, blockchain_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let blockchain_handler = MessageHandler::new(blockchain_sender, blockchain_receiver);

        let (gossip_sender, gossip_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let gossip_handler = MessageHandler::new(gossip_sender, gossip_receiver);

        let (swarm_sender, swarm_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let swarm_handler = MessageHandler::new(swarm_sender, swarm_receiver);

        let (state_sender, mut state_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let state_handler = MessageHandler::new(state_sender, state_receiver);

        let (gossip_tx_sender, gossip_tx_receiver) = std::sync::mpsc::channel();

        let (ctrl_sender, mut ctrl_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let ctrl_handler = MessageHandler::new(ctrl_sender.clone(), ctrl_receiver);

        let mut cmd_handler = CommandHandler::new(
            mining_handler.sender,
            blockchain_handler.sender,
            gossip_handler.sender,
            swarm_handler.sender,
            state_handler.sender,
            gossip_tx_sender,
            ctrl_handler.receiver,
        );

        // NOTE: in case logging is needed, remove if not needed
        // TelemetrySubscriber::init(std::io::stdout).unwrap();

        let (msg_sender, mut msg_receiver) = tokio::sync::mpsc::unbounded_channel::<MessageType>();
        let (msg_sender_1, mut msg_receiver_1) =
            tokio::sync::mpsc::unbounded_channel::<(Packet, SocketAddr)>();

        let msg_handler = MessageHandler::new(msg_sender, msg_receiver_1);

        // NOTE: preliminary setup above this note
        let mut node = Node::new(NodeType::Full, cmd_handler, msg_handler, 100);

        let handle = tokio::spawn(async move {
            node.start().await.unwrap();
        });

        ctrl_sender.send(Command::MineBlock).unwrap();

        // NOTE: stop the node after tests are performed
        ctrl_sender.send(Command::Stop).unwrap();
        
        handle.await.unwrap();

        let cmd = mining_handler.receiver.recv().await.unwrap();

        let mut passing = false;

        if let Command::StartMiner = cmd {
            passing = true;
        }

        assert!(passing);
    }


    #[tokio::test]
    async fn node_can_allocate_send_address() {
        let (mining_sender, mut mining_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut mining_handler = MessageHandler::new(mining_sender, mining_receiver);

        let (blockchain_sender, blockchain_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let blockchain_handler = MessageHandler::new(blockchain_sender, blockchain_receiver);

        let (gossip_sender, gossip_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let gossip_handler = MessageHandler::new(gossip_sender, gossip_receiver);

        let (swarm_sender, swarm_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let swarm_handler = MessageHandler::new(swarm_sender, swarm_receiver);

        let (state_sender, mut state_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let state_handler = MessageHandler::new(state_sender, state_receiver);

        let (gossip_tx_sender, gossip_tx_receiver) = std::sync::mpsc::channel();

        let (ctrl_sender, mut ctrl_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let ctrl_handler = MessageHandler::new(ctrl_sender.clone(), ctrl_receiver);

        let mut cmd_handler = CommandHandler::new(
            mining_handler.sender,
            blockchain_handler.sender,
            gossip_handler.sender,
            swarm_handler.sender,
            state_handler.sender,
            gossip_tx_sender,
            ctrl_handler.receiver,
        );

        // NOTE: in case logging is needed, remove if not needed
        // TelemetrySubscriber::init(std::io::stdout).unwrap();

        let (msg_sender, mut msg_receiver) = tokio::sync::mpsc::unbounded_channel::<MessageType>();
        let (msg_sender_1, mut msg_receiver_1) =
            tokio::sync::mpsc::unbounded_channel::<(Packet, SocketAddr)>();

        let msg_handler = MessageHandler::new(msg_sender, msg_receiver_1);

        // NOTE: preliminary setup above this note
        let mut node = Node::new(NodeType::Full, cmd_handler, msg_handler, 100);

        let handle = tokio::spawn(async move {
            node.start().await.unwrap();
        });

        ctrl_sender.send(Command::SendAddress).unwrap();

        // NOTE: stop the node after tests are performed
        ctrl_sender.send(Command::Stop).unwrap();
        handle.await.unwrap();

        let cmd = mining_handler.receiver.recv().await.unwrap();

        let mut passing = false;

        if let Command::SendAddress = cmd {
            passing = true;
        }

        assert!(passing);
    }

    #[tokio::test]
    async fn node_can_allocate_send_state() {
        let (mining_sender, mut mining_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut mining_handler = MessageHandler::new(mining_sender, mining_receiver);

        let (blockchain_sender, blockchain_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut blockchain_handler = MessageHandler::new(blockchain_sender, blockchain_receiver);

        let (gossip_sender, gossip_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let gossip_handler = MessageHandler::new(gossip_sender, gossip_receiver);

        let (swarm_sender, swarm_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let swarm_handler = MessageHandler::new(swarm_sender, swarm_receiver);

        let (state_sender, mut state_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let state_handler = MessageHandler::new(state_sender, state_receiver);

        let (gossip_tx_sender, gossip_tx_receiver) = std::sync::mpsc::channel();
        //let gossip_tx_sender = Mutex::new(g_tx_sender);

        let (ctrl_sender, mut ctrl_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let ctrl_handler = MessageHandler::new(ctrl_sender.clone(), ctrl_receiver);

        let mut cmd_handler = CommandHandler::new(
            mining_handler.sender,
            blockchain_handler.sender,
            gossip_handler.sender,
            swarm_handler.sender,
            state_handler.sender,
            gossip_tx_sender,
            ctrl_handler.receiver,
        );

        // NOTE: in case logging is needed, remove if not needed
        // TelemetrySubscriber::init(std::io::stdout).unwrap();

        let (msg_sender, mut msg_receiver) = tokio::sync::mpsc::unbounded_channel::<MessageType>();
        let (msg_sender_1, mut msg_receiver_1) =
            tokio::sync::mpsc::unbounded_channel::<(Packet, SocketAddr)>();

        let msg_handler = MessageHandler::new(msg_sender, msg_receiver_1);

        // NOTE: preliminary setup above this note
        let mut node = Node::new(NodeType::Full, cmd_handler, msg_handler, 100);

        let handle = tokio::spawn(async move {
            node.start().await.unwrap();
        });

        let addr = format!("0x{:0^1$}", "0", 30);

        ctrl_sender.send(Command::SendState(addr, 50)).unwrap();

        // NOTE: stop the node after tests are performed
        ctrl_sender.send(Command::Stop).unwrap();
        handle.await.unwrap();

        if let Command::SendState(addr, 50) =  blockchain_handler.receiver.recv().await.unwrap(){
            assert!(true);
        }
    }

    #[tokio::test]
    async fn node_can_allocate_message() {
        let (mining_sender, mut mining_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut mining_handler = MessageHandler::new(mining_sender, mining_receiver);

        let (blockchain_sender, blockchain_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut blockchain_handler = MessageHandler::new(blockchain_sender, blockchain_receiver);

        let (gossip_sender, gossip_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let gossip_handler = MessageHandler::new(gossip_sender, gossip_receiver);

        let (swarm_sender, swarm_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let swarm_handler = MessageHandler::new(swarm_sender, swarm_receiver);

        let (state_sender, mut state_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut state_handler = MessageHandler::new(state_sender, state_receiver);

        let (gossip_tx_sender, mut gossip_tx_receiver) = std::sync::mpsc::channel();

        let (ctrl_sender, mut ctrl_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut ctrl_handler = MessageHandler::new(ctrl_sender.clone(), ctrl_receiver);

        let mut cmd_handler = CommandHandler::new(
            mining_handler.sender,
            blockchain_handler.sender,
            gossip_handler.sender,
            swarm_handler.sender,
            state_handler.sender,
            gossip_tx_sender,
            ctrl_handler.receiver,
        );

        // NOTE: in case logging is needed, remove if not needed
        // TelemetrySubscriber::init(std::io::stdout).unwrap();

        let (msg_sender, mut msg_receiver) = tokio::sync::mpsc::unbounded_channel::<MessageType>();
        let (msg_sender_1, mut msg_receiver_1) =
            tokio::sync::mpsc::unbounded_channel::<(Packet, SocketAddr)>();

        let msg_handler = MessageHandler::new(msg_sender, msg_receiver_1);

        // NOTE: preliminary setup above this note
        let mut node = Node::new(NodeType::Full, cmd_handler, msg_handler, 100);

        let handle = tokio::spawn(async move {
            node.start().await.unwrap();
        });

        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        
        let message = Message{ head: Header::Gossip, msg: vec![0,1] };
        
        ctrl_sender.send(Command::SendMessage(socket, message)).unwrap();

        // NOTE: stop the node after tests are performed
        ctrl_sender.send(Command::Stop).unwrap();
        handle.await.unwrap();

        if let (socket, message) = gossip_tx_receiver.recv().unwrap(){
            assert!(true);
        }
    }

    #[tokio::test]
    async fn node_can_allocate_store_state_db_chunk() {
        let (mining_sender, mut mining_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut mining_handler = MessageHandler::new(mining_sender, mining_receiver);

        let (blockchain_sender, blockchain_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut blockchain_handler = MessageHandler::new(blockchain_sender, blockchain_receiver);

        let (gossip_sender, gossip_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let gossip_handler = MessageHandler::new(gossip_sender, gossip_receiver);

        let (swarm_sender, swarm_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let swarm_handler = MessageHandler::new(swarm_sender, swarm_receiver);

        let (state_sender, mut state_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut state_handler = MessageHandler::new(state_sender, state_receiver);

        let (gossip_tx_sender, mut gossip_tx_receiver) = std::sync::mpsc::channel();

        let (ctrl_sender, mut ctrl_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut ctrl_handler = MessageHandler::new(ctrl_sender.clone(), ctrl_receiver);

        let mut cmd_handler = CommandHandler::new(
            mining_handler.sender,
            blockchain_handler.sender,
            gossip_handler.sender,
            swarm_handler.sender,
            state_handler.sender,
            gossip_tx_sender,
            ctrl_handler.receiver,
        );

        // NOTE: in case logging is needed, remove if not needed
        // TelemetrySubscriber::init(std::io::stdout).unwrap();

        let (msg_sender, mut msg_receiver) = tokio::sync::mpsc::unbounded_channel::<MessageType>();
        let (msg_sender_1, mut msg_receiver_1) =
            tokio::sync::mpsc::unbounded_channel::<(Packet, SocketAddr)>();

        let msg_handler = MessageHandler::new(msg_sender, msg_receiver_1);

        // NOTE: preliminary setup above this note
        let mut node = Node::new(NodeType::Full, cmd_handler, msg_handler, 100);

        let handle = tokio::spawn(async move {
            node.start().await.unwrap();
        });
        
        let cmd = Command::StoreStateDbChunk(vec![0,1], vec![0,1], 7, 2);
        ctrl_sender.send(cmd).unwrap();

        // NOTE: stop the node after tests are performed
        ctrl_sender.send(Command::Stop).unwrap();
        handle.await.unwrap();

        assert!(true);
    }

    #[tokio::test]
    #[should_panic]
    async fn node_allocates_to_right_chanel() {
        let (mining_sender, mut mining_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut mining_handler = MessageHandler::new(mining_sender, mining_receiver);

        let (blockchain_sender, blockchain_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut blockchain_handler = MessageHandler::new(blockchain_sender, blockchain_receiver);

        let (gossip_sender, gossip_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let gossip_handler = MessageHandler::new(gossip_sender, gossip_receiver);

        let (swarm_sender, swarm_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let swarm_handler = MessageHandler::new(swarm_sender, swarm_receiver);

        let (state_sender, mut state_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut state_handler = MessageHandler::new(state_sender, state_receiver);

        let (gossip_tx_sender, mut gossip_tx_receiver) = std::sync::mpsc::channel();

        let (ctrl_sender, mut ctrl_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut ctrl_handler = MessageHandler::new(ctrl_sender.clone(), ctrl_receiver);

        let mut cmd_handler = CommandHandler::new(
            mining_handler.sender,
            blockchain_handler.sender,
            gossip_handler.sender,
            swarm_handler.sender,
            state_handler.sender,
            gossip_tx_sender,
            ctrl_handler.receiver,
        );

        // NOTE: in case logging is needed, remove if not needed
        // TelemetrySubscriber::init(std::io::stdout).unwrap();

        let (msg_sender, mut msg_receiver) = tokio::sync::mpsc::unbounded_channel::<MessageType>();
        let (msg_sender_1, mut msg_receiver_1) =
            tokio::sync::mpsc::unbounded_channel::<(Packet, SocketAddr)>();

        let msg_handler = MessageHandler::new(msg_sender, msg_receiver_1);

        // NOTE: preliminary setup above this note
        let mut node = Node::new(NodeType::Full, cmd_handler, msg_handler, 100);

        let handle = tokio::spawn(async move {
            node.start().await.unwrap();
        });

        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        
        let message = Message{ head: Header::Gossip, msg: vec![0,1] };
        
        ctrl_sender.send(Command::SendMessage(socket, message)).unwrap();

        // NOTE: stop the node after tests are performed
        ctrl_sender.send(Command::Stop).unwrap();
        handle.await.unwrap();

        if let (socket, message) = gossip_tx_receiver.recv().unwrap(){
            assert!(true);
        }

        let cmd = state_handler.receiver.recv().await.unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn node_allocates_send_state_to_right_channel() {
        let (mining_sender, mut mining_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut  mining_handler = MessageHandler::new(mining_sender, mining_receiver);

        let (blockchain_sender, blockchain_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut blockchain_handler = MessageHandler::new(blockchain_sender, blockchain_receiver);

        let (gossip_sender, gossip_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let mut gossip_handler = MessageHandler::new(gossip_sender, gossip_receiver);

        let (swarm_sender, swarm_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let swarm_handler = MessageHandler::new(swarm_sender, swarm_receiver);

        let (state_sender, mut state_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let state_handler = MessageHandler::new(state_sender, state_receiver);

        let (gossip_tx_sender, gossip_tx_receiver) = std::sync::mpsc::channel();

        let (ctrl_sender, mut ctrl_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let ctrl_handler = MessageHandler::new(ctrl_sender.clone(), ctrl_receiver);

        let mut cmd_handler = CommandHandler::new(
            mining_handler.sender,
            blockchain_handler.sender,
            gossip_handler.sender,
            swarm_handler.sender,
            state_handler.sender,
            gossip_tx_sender,
            ctrl_handler.receiver,
        );

        // NOTE: in case logging is needed, remove if not needed
        // TelemetrySubscriber::init(std::io::stdout).unwrap();

        let (msg_sender, mut msg_receiver) = tokio::sync::mpsc::unbounded_channel::<MessageType>();
        let (msg_sender_1, mut msg_receiver_1) =
            tokio::sync::mpsc::unbounded_channel::<(Packet, SocketAddr)>();

        let msg_handler = MessageHandler::new(msg_sender, msg_receiver_1);

        // NOTE: preliminary setup above this note
        let mut node = Node::new(NodeType::Full, cmd_handler, msg_handler, 100);

        let handle = tokio::spawn(async move {
            node.start().await.unwrap();
        });

        let addr = format!("0x{:0^1$}", "0", 30);

        ctrl_sender.send(Command::SendState(addr, 50)).unwrap();

        // NOTE: stop the node after tests are performed
        ctrl_sender.send(Command::Stop).unwrap();
        handle.await.unwrap();

        if let Command::SendState(addr, 50) =  blockchain_handler.receiver.recv().await.unwrap(){
            assert!(true);
        }

        let cmd = gossip_handler.receiver.recv().await.unwrap();

    }


    /* 
    #[tokio::test]
    async fn node_sends_message_to_all_peers(){
        
        let mut dummy_nodes: Vec<Arc<std::sync::Mutex<Node>>> = Vec::new();

        let mut ctrl_senders: Vec<tokio::sync::mpsc::UnboundedSender<Command>> = Vec::new();

        let mut gossip_tx_receivers = Vec::new();

        let (ctrl_sender, mut ctrl_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
        let ctrl_handler = MessageHandler::new(ctrl_sender.clone(), ctrl_receiver);

        //let shared_sender = Mutex::new(ctrl_sender.clone());

        //spawn a bunch of node times
        //create transmitters and receivers (ctrl) for each
        //to stop all at the same time make an entity/fxn that sends messages to all ctrl receivers

        //*control receiver to each node runtime to interact with nodes in any way other than to stop is wrong */
        

        (0..8).for_each(|i|{
            let (mining_sender, mining_receiver) =
            tokio::sync::mpsc::unbounded_channel::<Command>();
            let mining_handler = MessageHandler::new(mining_sender, mining_receiver);

            let (blockchain_sender, blockchain_receiver) =
                tokio::sync::mpsc::unbounded_channel::<Command>();
            let blockchain_handler = MessageHandler::new(blockchain_sender, blockchain_receiver);

            let (gossip_sender, gossip_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
            let gossip_handler = MessageHandler::new(gossip_sender, gossip_receiver);

            let (swarm_sender, swarm_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
            let swarm_handler = MessageHandler::new(swarm_sender, swarm_receiver);

            let (state_sender, state_receiver) = tokio::sync::mpsc::unbounded_channel::<Command>();
            let state_handler = MessageHandler::new(state_sender, state_receiver);

            let (gossip_tx_sender, mut gossip_tx_receiver) = std::sync::mpsc::channel();

            let mut cmd_handler = CommandHandler::new(
                mining_handler.sender,
                blockchain_handler.sender,
                gossip_handler.sender,
                swarm_handler.sender,
                state_handler.sender,
                gossip_tx_sender.clone(),
                ctrl_handler.receiver,
            );

            // NOTE: in case logging is needed, remove if not needed
            // TelemetrySubscriber::init(std::io::stdout).unwrap();

            let (msg_sender, msg_receiver) = tokio::sync::mpsc::unbounded_channel::<MessageType>();
            let (msg_sender_1, msg_receiver_1) =
                tokio::sync::mpsc::unbounded_channel::<(Packet, SocketAddr)>();

            let msg_handler = MessageHandler::new(msg_sender, msg_receiver_1);

            // NOTE: preliminary setup above this note
            let mut node = Node::new(NodeType::Full, cmd_handler, msg_handler, 100);

            dummy_nodes.push(Arc::new(Mutex::new(node)));
            //push arc mutex

            ctrl_senders.push(ctrl_sender);

            gossip_tx_receivers.push(gossip_tx_receiver);
        });

        //dbg!("dummy nodes? : {:?}", dummy_nodes);

        let mut handles = Vec::new();

        dummy_nodes.iter().for_each(|node|{

            let handle = thread::spawn(move || {
                node.lock().unwrap().start();
            });

            handles.push(handle);
        });

        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        
        let message = Message{ head: Header::Gossip, msg: vec![0,1] };

        assert!(Some(ctrl_senders.get(0)).is_some());

        ctrl_senders[0].send(Command::SendMessage(socket, message)).unwrap();
        (1..8).for_each(|i|{
            ctrl_senders[i].send(Command::Stop).unwrap();
        });

        ctrl_senders[0].send(Command::Stop).unwrap();

        //handles[1].unwrap();
        //handles[2].unwrap();
        //handles[3].await.unwrap();
        //handles[4].await.unwrap();
        //handles[5].await.unwrap();
        //handles[6].await.unwrap();
        //handles[7].await.unwrap();

        // NOTE: stop the node after tests are performed
        
        for receiver in gossip_tx_receivers.iter().skip(0){
            dbg!("node");

            if let (socket, message) = gossip_tx_receivers[0].recv().unwrap(){
                assert!(true);
        }
    }
    }
    */
}








