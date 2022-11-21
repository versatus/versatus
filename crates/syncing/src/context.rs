use std::{
    borrow::Borrow,
    net::{IpAddr, Ipv4Addr},
    path::Path,
    process,
};

use clap::Parser;
use config::{Config, File};
use primitives::types::NodeType;
use rcrefcell::RcCell;
use telemetry::{debug, error};
use uuid::Uuid;

use crate::{config::AppConfig, lrnodepool::LeftRightNodePoolDB};

/// Dispersed routes discovery cluster app. Simplified to enable easier cluster
/// fault detection.
#[derive(Parser, Debug, Clone)]
#[command(version)]
pub struct AppParams {
    /// Local bind address like "0.0.0.0"
    #[arg(short, long, default_value_t = String::from("0.0.0.0"))]
    pub local_bind_address: String,

    /// Broadcast address like "255.255.255.255"
    #[arg(short, long, default_value_t = String::from("255.255.255.255"))]
    pub broadcast_address: String,

    /// Broadcast address like "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8"
    #[arg(short, long, default_value_t = String::from(""))]
    pub uuid_node: String,

    /// localstate path
    #[arg(short, long, default_value_t = String::from("./test_chain.db"))]
    pub file_localstate: String,

    /// localstate path
    #[arg(short, long, default_value_t = String::from("route"))]
    pub config: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum NodeUpdateState {
    UpToDate = 0,
    InProgress = 1,
    Invalid = 100,
}

pub struct BrokerAddr {
    local_ip: IpAddr,
    port: u16,
}

impl BrokerAddr {
    pub fn new(local_ip: IpAddr, port: u16) -> Self {
        BrokerAddr { local_ip, port }
    }

    // TODO, problems with compiler.
    #[allow(dead_code)]
    pub fn new_from_str(local_ip: String, port: String) -> Self {
        let local_ip = match local_ip.parse::<IpAddr>() {
            Ok(ip) => ip,
            Err(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
        };

        let local_port = port.parse::<u16>().unwrap_or(9000);


        BrokerAddr {
            local_ip,
            port: local_port,
        }
    }

    pub fn local_ip(&self) -> &IpAddr {
        &self.local_ip
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

/// Application context, an alternative of using global static vars or passing
/// around tens of parameters.
pub struct AppContext<'m> {
    pub node_routes_db: LeftRightNodePoolDB<'m>,
    pub args: AppParams,
    pub node_type: NodeType,
    pub node_id: String,
    pub node_state: NodeUpdateState,
    pub localstate_file_path: String,
    pub bind_sender_local_address: String, // "0.0.0.0" or specific interface 192.168.1.10
    pub broadcast_target_address: String,  /* "255.255.255.255" or specific sub network
                                            * 192.168.255.255 */
    pub discovery_port: u16, // common discovery port
    pub broker_edge: BrokerAddr,
}

pub type RcContext<'m> = RcCell<AppContext<'m>>;

/// Application context handler, encapsulating the AppContext with handy helper
/// functions.
pub struct ContextHandler<'m> {
    state: RcContext<'m>,
}

unsafe impl<'m> Send for ContextHandler<'m> {}

impl<'m> ContextHandler<'m> {
    /// Localstate data broker, all the operation in a separate thread.
    /// TODO: To be integrated with the existing structure.
    ///
    /// # Arguments
    /// * `node_type`                   - Node type.
    /// * `discovery_local_address`     - local bind address for discovery, by
    ///   default 0.0.0.0
    /// * `discovery_broadcast_address` - broadcast address for discovery, by
    ///   default 0.0.0.0
    /// * `discovery_port`              - discovery port
    /// * `broker_local_ip`             - broker local bind address 0.0.0.0
    /// * `broker_port`                 - broker port
    pub fn _new_with_params(
        node_type: NodeType,
        discovery_local_address: &str,
        discovery_broadcast_address: &str,
        discovery_port: u16,
        broker_local_ip: &str,
        broker_port: &str,
    ) -> Self
    where
        AppContext<'m>: Send + 'static,
    {
        ContextHandler {
            state: AppContext::<'m>::new_with_params(
                node_type,
                discovery_local_address,
                discovery_broadcast_address,
                discovery_port,
                broker_local_ip,
                broker_port,
            ),
        }
    }

    /// Context wrapper builder from exisiting context.
    ///
    /// # Arguments
    /// * `context`             - application context with predefined parameters
    pub fn new(context: RcContext<'m>) -> Self {
        ContextHandler { state: context }
    }

    /// Context wrapper builder from scratch.
    pub fn init() -> Self
    where
        AppContext<'m>: Send + 'static,
    {
        ContextHandler {
            state: AppContext::<'m>::new(),
        }
    }

    /// Context wrapper builder from exisiting config.
    ///
    /// # Arguments
    /// * `context`             - application context with predefined parameters
    pub fn _new_from_config(config: AppConfig) -> Self
    where
        AppContext<'m>: Send + 'static,
    {
        ContextHandler {
            state: AppContext::<'m>::new_from_config(config),
        }
    }

    /// Create a clone of the handler with same application context.
    pub fn clone(&self) -> Self {
        ContextHandler::new(self.state.clone())
    }

    pub fn get(&self) -> RcContext<'m> {
        self.state.clone()
    }
}

impl<'m> AppContext<'m> {
    /// Create AppContext from cli parameters.
    pub fn new() -> RcCell<Self>
    where
        AppContext<'m>: Send + 'static,
    {
        let args = AppParams::parse();

        let conf = Config::builder()
            // .add_source(File::with_name("bootstrap_default.toml"))
            .add_source(File::with_name(args.config.as_str()).required(false))
            .build();

        let config: AppConfig = match conf {
            Ok(c) => match c.try_deserialize() {
                Ok(cfg) => cfg,
                Err(e) => {
                    error!("Configuration parse error : {:#?}", e);
                    process::exit(0x0100);
                },
            },
            Err(e) => {
                error!("Configuration parse error : {:#?}", e);
                process::exit(0x0100);
            },
        };

        debug!("Loaded configuration : {:#?}", config);

        let node_id = match Uuid::parse_str(config.node_id.as_str()) {
            Ok(uuid) => uuid.to_string(),
            Err(e) => {
                error!("UUID Parse error: {}", e);
                Uuid::new_v4().to_string()
            },
        };

        let localstate_filepath = &config.file_path_localstate;
        if !Path::new(&localstate_filepath).exists() {
            error!("Incorrect localstate file path : {}", &localstate_filepath);
            // TODO: Exit strategy
            process::exit(0x0100);
        }

        let broker = BrokerAddr::new(config.broker_local_ip, config.broker_port);

        RcCell::new(AppContext {
            node_routes_db: LeftRightNodePoolDB::<'m>::new(),
            args,
            node_type: config.node_type,
            node_id,
            node_state: if config.is_node_origin {
                NodeUpdateState::UpToDate
            } else {
                NodeUpdateState::InProgress
            },
            localstate_file_path: localstate_filepath.to_string(),
            bind_sender_local_address: config.discovery_bind_local_address.to_string(),
            broadcast_target_address: config.discovery_broadcast_address.to_string(),
            discovery_port: config.discovery_port,
            broker_edge: broker,
        })
    }

    /// Create context from already exising configuration file.
    ///
    /// # Arguments
    /// * `config`             - application configuration
    // TODO
    #[allow(dead_code)]
    pub fn new_from_config(config: AppConfig) -> RcCell<Self>
    where
        AppContext<'m>: Send + 'static,
    {
        let args = AppParams::parse();

        let node_id = match Uuid::parse_str(config.node_id.as_str()) {
            Ok(uuid) => uuid.to_string(),
            Err(e) => {
                error!("UUID Parse error: {}", e);
                Uuid::new_v4().to_string()
            },
        };

        let localstate_filepath = &config.file_path_localstate;
        if !Path::new(&localstate_filepath).exists() {
            error!("Incorrect localstate file path : {}", &localstate_filepath);
            // TODO: Exit strategy
            process::exit(0x0100);
        }

        let broker = BrokerAddr::new(config.broker_local_ip, config.broker_port);

        RcCell::new(AppContext {
            node_routes_db: LeftRightNodePoolDB::<'m>::new(),
            args,
            node_type: config.node_type,
            node_id,
            node_state: if config.is_node_origin {
                NodeUpdateState::UpToDate
            } else {
                NodeUpdateState::InProgress
            },
            localstate_file_path: localstate_filepath.to_string(),
            bind_sender_local_address: config.discovery_bind_local_address.to_string(),
            broadcast_target_address: config.discovery_broadcast_address.to_string(),
            discovery_port: config.discovery_port,
            broker_edge: broker,
        })
    }

    ///
    /// Create context with parameters.
    ///
    /// # Arguments
    ///
    /// * `node_type`                   - Node type.
    /// * `discovery_local_address`     - local bind address for discovery, by
    ///   default 0.0.0.0
    /// * `discovery_broadcast_address` - broadcast address for discovery, by
    ///   default 0.0.0.0
    /// * `discovery_port`              - discovery port
    /// * `broker_local_ip`             - broker local bind address 0.0.0.0
    /// * `broker_port`                 - broker port
    // TODO
    #[allow(dead_code)]
    pub fn new_with_params(
        node_type: NodeType,
        discovery_bind_local_address: &str,
        discovery_broadcast_address: &str,
        discovery_port: u16,
        broker_local_ip: &str,
        broker_port: &str,
    ) -> RcCell<Self>
    where
        AppContext<'m>: Send + 'static,
    {
        let args = AppParams::parse();

        let node_id = match Uuid::parse_str(args.uuid_node.as_str()) {
            Ok(uuid) => uuid.to_string(),
            Err(e) => {
                error!("UUID Parse error: {}", e);
                Uuid::new_v4().to_string()
            },
        };

        let localstate_filepath = &args.file_localstate;
        if !Path::new(&localstate_filepath).exists() {
            error!("Incorrect localstate file path : {}", &localstate_filepath);
            // TODO: Exit strategy
            process::exit(0x0100);
        }

        let broker = BrokerAddr::new_from_str(broker_local_ip.to_string(), broker_port.to_string());

        RcCell::new(AppContext {
            node_routes_db: LeftRightNodePoolDB::<'m>::new(),
            args: args.clone(),
            node_type,
            node_id,
            node_state: NodeUpdateState::InProgress,
            localstate_file_path: localstate_filepath.to_string(),
            bind_sender_local_address: discovery_bind_local_address.to_string(),
            broadcast_target_address: discovery_broadcast_address.to_string(),
            discovery_port,
            broker_edge: broker,
        })
    }

    /// Setters & Getters section.
    pub fn set_state(&mut self, new_state: NodeUpdateState) -> &Self {
        self.node_state = new_state;
        self
    }

    pub fn state(&self) -> &NodeUpdateState {
        &self.node_state
    }

    pub fn _get(&self) -> &AppContext<'m> {
        self.borrow()
    }
}
