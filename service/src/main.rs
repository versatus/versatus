use std::io;

/// Service is responsible for initializing the node, handling networking and config management
struct Service {
    //
}

impl Service {
    pub fn start() {
        // setup ports and control channels and loops
    }
}

/// Main entrypoint
fn main() -> Result<(), impl Error> {
    telemetry::TelemetrySubscriber::init(io::stdout)?;
}
