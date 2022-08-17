// Feature Tag(s): Validator Stake Calculation, Tx Validation, Tx Confirmation, Masternode Signing, Masternode Election
// Node Reputation Scores, Block Structure, Packet Processing, Message Processing, Message Allocating, Message Caching
use serde::{Deserialize, Serialize};
use messages::packet::Packet;
use std::net::SocketAddr;
use messages::message_types::MessageType;
use udp2p::protocol::protocol::Message;

/// Basic Command Constants
//TODO: Need to add all potential input commands
pub const SENDTXN: &str = "SENDTXN";
pub const GETBAL: &str = "GETBAL";
pub const GETSTATE: &str = "GETSTATE";
pub const MINEBLOCK: &str = "MINEBLK";
pub const SENDADDRESS: &str = "SENDADR";
pub const STOPMINE: &str = "STOPMINE";
pub const GETHEIGHT: &str = "GETHEIGHT";
pub const QUIT: &str = "QUIT";

/// Component Types of a state update
#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub enum ComponentTypes {
    Genesis,
    Child,
    Parent,
    Blockchain,
    Ledger,
    NetworkState,
    Archive,
    All,
}

/// The command enum is the basic datatype used to send commands around the program
//TODO: Review all the commands and determine which ones are needed, which can be changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    //TODO: Replace standard types with custom types for better readability
    // and to help engineers understand what the hell these items are. 
    SendTxn(u32, String, u128), // address number, receiver address, amount
    ProcessTxn(Vec<u8>),
    ProcessTxnValidator(Vec<u8>),
    ConfirmedBlock(Vec<u8>),
    PendingBlock(Vec<u8>, String),
    InvalidBlock(Vec<u8>),
    ProcessClaim(Vec<u8>),
    CheckStateUpdateStatus((u128, Vec<u8>, u128)),
    StateUpdateCompleted(Vec<u8>),
    StoreStateDbChunk(Vec<u8>, Vec<u8>, u32, u32),
    SendState(String, u128),
    SendMessage(SocketAddr, Message),
    GetBalance(u32),
    SendGenesis(String),
    SendStateComponents(String, Vec<u8>, String),
    GetStateComponents(String, Vec<u8>, String),
    RequestedComponents(String, Vec<u8>, String, String),
    StoreStateComponents(Vec<u8>, ComponentTypes),
    StoreChild(Vec<u8>),
    StoreParent(Vec<u8>),
    StoreGenesis(Vec<u8>),
    StoreLedger(Vec<u8>),
    StoreNetworkState(Vec<u8>),
    StateUpdateComponents(Vec<u8>, ComponentTypes),
    UpdateLastBlock(Vec<u8>),
    ClaimAbandoned(String, Vec<u8>),
    SlashClaims(Vec<String>),
    UpdateAppMiner(Vec<u8>),
    UpdateAppBlockchain(Vec<u8>),
    UpdateAppMessageCache(Vec<u8>),
    UpdateAppWallet(Vec<u8>),
    Publish(Vec<u8>),
    Gossip(Vec<u8>),
    AddNewPeer(String, String),
    AddKnownPeers(Vec<u8>),
    AddExplicitPeer(String, String),
    ProcessPacket((Packet, SocketAddr)),
    Bootstrap(String, String),
    SendPing(String),
    ReturnPong(Vec<u8>, String),
    InitHandshake(String),
    ReciprocateHandshake(String, String, String),
    CompleteHandshake(String, String, String),
    ProcessAck(String, u32, String),
    CleanInbox(String),
    CheckAbandoned,
    StartMiner,
    GetHeight,
    MineBlock,
    MineGenesis,
    StopMine,
    GetState,
    ProcessBacklog,
    SendAddress,
    NonceUp,
    Quit,
}

/// A Trait to convert different types into a command
pub trait AsCommand {
    fn into_command(&self) -> Command;
}

impl Command {
    /// Converts a string (typically a user input in the terminal interface) into a command
    pub fn from_str(command_string: &str) -> Option<Command> {
        let args: Vec<&str> = command_string.split(' ').collect();
        if args.len() == 4 {
            match args[0] {
                SENDTXN => {
                    return Some(Command::SendTxn(
                        args[1].parse::<u32>().unwrap(),
                        args[2].to_string(),
                        args[3].parse::<u128>().unwrap(),
                    ))
                }
                _ => {
                    println!("Invalid command string!");
                    return None;
                }
            }
        } else if args.len() == 3 {
            match args[0] {
                _ => {
                    println!("Invalid command string!");
                    return None;
                }
            }
        } else if args.len() == 2 {
            match args[0] {
                GETBAL => {
                    if let Ok(num) = args[1].parse::<u32>() {
                        return Some(Command::GetBalance(num));
                    } else {
                        println!("Invalid command string");
                        None
                    }
                }
                _ => {
                    println!("Invalid command string");
                    None
                }
            }
        } else {
            match command_string.clone() {
                GETSTATE => return Some(Command::GetState),
                MINEBLOCK => return Some(Command::MineBlock),
                STOPMINE => return Some(Command::StopMine),
                SENDADDRESS => return Some(Command::SendAddress),
                GETHEIGHT => return Some(Command::GetHeight),
                QUIT => return Some(Command::Quit),
                _ => {
                    println!("Invalid command string");
                    None
                }
            }
        }
    }
}

impl ComponentTypes {
    /// Converts a Componenet type into an integer.
    pub fn to_int(&self) -> u8 {
        match *self {
            ComponentTypes::Genesis => 0,
            ComponentTypes::Child => 1,
            ComponentTypes::Parent => 2,
            ComponentTypes::NetworkState => 3,
            ComponentTypes::Ledger => 4,
            ComponentTypes::Blockchain => 5,
            ComponentTypes::Archive => 6,
            ComponentTypes::All => 7,
        }
    }
}

impl PartialEq for ComponentTypes {
    fn eq(&self, other: &Self) -> bool {
        self.to_int() == other.to_int()
    }
}

impl Eq for ComponentTypes {}