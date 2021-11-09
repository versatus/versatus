use serde::{Deserialize, Serialize};
use messages::packet::Packet;
use std::net::SocketAddr;

pub const SENDTXN: &str = "SENDTXN";
pub const GETBAL: &str = "GETBAL";
pub const GETSTATE: &str = "GETSTATE";
pub const MINEBLOCK: &str = "MINEBLK";
pub const SENDADDRESS: &str = "SENDADR";
pub const STOPMINE: &str = "STOPMINE";
pub const GETHEIGHT: &str = "GETHEIGHT";
pub const QUIT: &str = "QUIT";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
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
    SendMessage(Vec<u8>),
    GetBalance(u32),
    SendGenesis(String),
    SendStateComponents(String, Vec<u8>),
    GetStateComponents(String, Vec<u8>),
    RequestedComponents(String, Vec<u8>),
    StoreStateComponents(Vec<u8>),
    StateUpdateComponents(Vec<u8>),
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
    SendPing(Vec<u8>),
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

pub trait AsCommand {
    fn into_command(&self) -> Command;
}

impl Command {
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
