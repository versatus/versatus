use routerswarm::network;
use routerswarm::context;

pub async fn route_discoverer() {
    
    let mut context = context::AppContext::new();
    
    match tokio::spawn(async move {
        network::routing_discoverer_start(&mut context).await;
    }).await {
        Ok(_) => log::info!("Route discoverer started"),
        Err(e) => log::error!("Thread starting error : {}", e)
    }
}
