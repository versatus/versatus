use thiserror::Error;

use tracing_subscriber::{
    fmt::MakeWriter,
    util::{SubscriberInitExt, TryInitError},
};

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("failed to initialize: {0}")]
    Init(#[from] TryInitError),

    #[error("{0}")]
    Other(String),

    #[error("unknown error occurred")]
    Unknown,
}

type Result<T> = std::result::Result<T, TelemetryError>;

// TODO: figure out the proper generic sig
#[derive(Debug)]
pub struct TelemetrySubscriber
// pub struct TelemetrySubscriber<S>
where
// S: Subscriber + SubscriberInitExt + Send + Sync,
// S: Subscriber + SubscriberInitExt + Send + Sync,
// S: Subscriber + Send + Sync,
// W: for<'s> MakeWriter<'s> + 'static,
// S: Subscriber + SubscriberInitExt + Send + Sync,
// S: Subscriber + SubscriberInitExt,
// S: Subscriber,
// S: Subscriber<JsonFields, Format<Json>, LevelFilter, W>,
{
    // sub: S,
}

impl TelemetrySubscriber {
    pub fn init<W>(out: W) -> Result<()>
    where
        W: for<'s> MakeWriter<'s> + 'static + Sync + Send,
    {
        let sub = tracing_subscriber::fmt()
            .with_writer(out)
            .with_file(true)
            .with_line_number(true)
            // .with_thread_ids(true)
            // .with_thread_names(true)
            .json()
            .finish();

        sub.try_init()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use tracing_subscriber::fmt::TestWriter;

    use super::*;

    #[test]
    fn it_works() {
        let tw = TestWriter::new();

        TelemetrySubscriber::init(tw).unwrap();
        tracing::info!("hello world 2");
    }
}
