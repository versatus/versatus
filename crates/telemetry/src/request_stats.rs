//! This is a module that tracks basic stats for servicing parts of complex requests. It's supposed
//! to be a little more granular than just the total time taken to service a request, but not as
//! granular as full-on execution profiling and without the special tools or overhead.
use log::info;
use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, StopWatchError>;

#[derive(Error, Debug)]
pub enum StopWatchError {
    #[error("StopWatch was never started")]
    FailedToStart,

    #[error("StopWatch wasn't stopped")]
    FailedToStop,

    #[error("StopWatch time went backwards")]
    CorruptedStartTime,

    #[error(transparent)]
    SystemTimeError(#[from] SystemTimeError),
}

/// Simple struct to track start/end times
#[derive(Debug)]
struct StopWatch {
    /// Start time for this specific stat.
    start_ms: Option<u128>,
    /// End time for this specific stat.
    stop_ms: Option<u128>,
}

impl StopWatch {
    /// Creates a new stopwatch object for tracking start/end times
    pub fn new() -> Result<Self> {
        Ok(StopWatch {
            start_ms: Some(Self::now()?),
            stop_ms: None,
        })
    }

    /// Private function for collecting the current timestamp
    fn now() -> Result<u128> {
        Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
    }

    /// Calculate the duration from the start and stop times.
    pub fn duration(&self) -> Result<u128> {
        let start: u128 = match self.start_ms {
            Some(val) => val,
            None => return Err(StopWatchError::FailedToStart),
        };

        let stop: u128 = match self.stop_ms {
            Some(val) => val,
            None => return Err(StopWatchError::FailedToStop),
        };

        if start > stop {
            return Err(StopWatchError::CorruptedStartTime);
        }
        let ret = stop - start;
        Ok(ret)
    }

    /// Stop a started stopwatch
    pub fn stop(&mut self) -> Result<()> {
        self.stop_ms = Some(Self::now()?);
        Ok(())
    }
}

#[derive(Debug)]
pub struct RequestStats {
    /// A label to identify this whole collection of stats. This is added to the default output,
    /// but has no special meaning. A good example might be to make this the module name.
    pub name: String,
    /// An instance ID for this specific set of stats. There is no special meaning to this
    /// attribute, but it is logged and a job UUID would be an example of a good choice here.
    pub instance: String,
    /// We maintain the stats internally in vectors for now because we're not expecting this to be
    /// used for a large number of stats within a given request. We also want to maintain the order
    /// that these are passed to us by the caller. The labels attribute is a set of string labels
    /// passed by the caller for them to identify stats in the output. It is assumed that these are
    /// unique within a given instance of [RequestStats].
    labels: Vec<String>,
    /// The stats values are stored in a second vector and we try to match these with the labels
    /// above.
    values: Vec<StopWatch>,
}

impl RequestStats {
    /// Creates a new [RequestStats] instance, with [name] and [instance] being strings provided by
    /// the caller to help them identify instances in any output.
    pub fn new(name: String, instance: String) -> Result<Self> {
        Ok(RequestStats {
            name,
            instance,
            labels: vec!["total".to_string()],
            values: vec![StopWatch::new()?],
        })
    }

    /// Starts measuring a new stat within this [RequestStats] instance.
    pub fn start(&mut self, name: String) -> Result<()> {
        self.labels.push(name);
        self.values.push(StopWatch::new()?);
        Ok(())
    }

    /// Stops measuring a started stat within this [RequestStats] instance. Only likely to fail in
    /// cases where we provide a named stat where the named stat doesn't already exist. The [name]
    /// parameter should refer to a stat created by [self.start()].
    pub fn stop(&mut self, name: String) -> Result<()> {
        if let Some(index) = self.labels.iter().position(|v| v == name.as_str()) {
            self.values[index].stop()?;
        }
        Ok(())
    }
}

impl Drop for RequestStats {
    /// The default destructor simply dumps a single line of text to the logging framework.
    fn drop(&mut self) {
        let _ = self.stop("total".to_string());
        let mut output = String::new();
        output += &format!("{}: {}", &self.name, &self.instance).to_string();
        for (i, stat) in self.labels.iter().enumerate() {
            if let Ok(val) = &self.values[i].duration() {
                output += &format!("; {}={} ", &stat, &val.to_string());
            }
        }
        info!("RequestStat: {}", output);
    }
}
