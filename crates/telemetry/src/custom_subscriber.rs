use primitives::{get_pretty_print_logs, Environment};
use thiserror::Error;
use tracing_subscriber::{
    fmt::MakeWriter,
    util::{SubscriberInitExt, TryInitError},
};

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("failed to initialize: {0}")]
    Init(#[from] TryInitError),
}

type Result<T> = std::result::Result<T, TelemetryError>;

// TODO: figure out the proper generic sig to export a telemetry builder instead
#[derive(Debug)]
pub struct TelemetrySubscriber {}

impl TelemetrySubscriber {
    pub fn init<W>(out: W) -> Result<()>
    where
        W: for<'s> MakeWriter<'s> + 'static + Sync + Send,
    {
        let environ = primitives::get_vrrb_environment();
        let is_local_env = matches!(environ, Environment::Local);

        let pretty_print_logs = get_pretty_print_logs();

        if pretty_print_logs {
            let sub = tracing_subscriber::fmt()
                .with_writer(out)
                .with_file(is_local_env)
                .with_line_number(is_local_env)
                .with_target(is_local_env)
                .compact()
                // .pretty()
                .with_max_level(tracing::Level::ERROR)
                .finish();

            sub.try_init()?;
        } else {
            let sub = tracing_subscriber::fmt()
                .with_writer(out)
                .with_file(is_local_env)
                .with_line_number(is_local_env)
                .json()
                .with_current_span(false)
                .flatten_event(true)
                .with_span_list(false)
                .finish();

            sub.try_init()?;
        }

        _set_panic_hook();

        Ok(())
    }
}

// TODO: Fix implementation of std::panic::set_hook
fn _set_panic_hook() {
    // std::panic::set_hook(Box::new(|panic_info| {
    //     let payload = panic_info.payload().downcast_ref::<&str>();
    //     let location = panic_info.location().map(|loc| loc.to_string());
    //
    //     tracing::error!(payload = payload, location = location);
    // }));
}

#[cfg(test)]
mod tests {

    use tracing_subscriber::fmt::TestWriter;

    use super::*;

    #[test]
    fn logs_to_stdout() {
        let tw = TestWriter::new();

        TelemetrySubscriber::init(tw).unwrap();

        tracing::info!("hello world 2");
    }
}
