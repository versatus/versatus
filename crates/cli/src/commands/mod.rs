use crate::{
    cli::{Args, Commands},
    result::{CliError, Result},
};

pub mod node;

pub async fn exec(args: Args) -> Result<()> {
    telemetry::debug!("args: {:?}", args);

    let cmd = args.command.unwrap_or_default();
    //error here at run node
    match cmd {
        //Commands::Wallet()
        Commands::Node(node_args) => node::exec(node_args).await,
        _ => Err(CliError::InvalidCommand(format!("{:?}", cmd))),
    }
}
