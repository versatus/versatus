use anyhow::Result;
use jsonrpsee::server::{ServerBuilder, ServerHandle};
use service_config::Config;

pub struct RpcServerListener;
impl RpcServerListener {
    /// Starts the RPC server.
    pub async fn start(
        config: &Config,
        service_name: &str,
        service_type: &str,
    ) -> Result<(ServerHandle, SocketAddress)> {
        let service_config = config.find_service(service_name, service_type)?;
        let server = ServerBuilder::default().build().await?;

        let addr = server.local_addr()?;
        let handle = server.start(server_impl.into_rpc())?;

        // TODO: refactor example out of here
        // In this example we don't care about doing shutdown so let's it run forever.
        // You may use the `ServerHandle` to shut it down or manage it yourself.
        Ok((handle, addr))
    }
}
