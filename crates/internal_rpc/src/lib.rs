use anyhow::Result;
use service_config::{Config, ServiceConfig};

pub struct InternalRpcManager;

impl InternalRpcManager {
    fn read_config(
        config: Config,
        service_name: &str,
        service_type: &str,
    ) -> Result<ServiceConfig> {
        config.find_service(service_name, service_type)
    }
    pub async fn listen(config: Config, service_name: &str, service_type: &str) -> Result<()> {
        let _service_config = InternalRpcManager::read_config(config, service_name, service_type)?;

        // listen for some event?
        Ok(())
    }
}
