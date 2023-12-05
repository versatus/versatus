use crate::render::RenderToPrometheus;
use hyper::server::conn::AddrIncoming;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use hyper_rustls::TlsAcceptor;
use platform::platform_stats::CgroupStats;
use prometheus::proto::MetricFamily;
use prometheus::{
    core::Collector, Counter, Encoder, Gauge, Histogram, HistogramOpts, IntCounter, IntGauge, Opts,
    Registry, TextEncoder,
};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::{fs, io};
use telemetry::{error, info};
use thiserror::Error;
use tokio::sync::mpsc::Receiver;

#[derive(Debug, Error)]
pub enum PrometheusFactoryError {
    #[error("Invalid Bind Address :{0}")]
    InvalidIpAddress(String),
    #[error("Failed to fetch metric  :{0}")]
    FailedToFetchMetric(String),
    #[error("Prometheus registration error: {0}")]
    RegistrationError(#[from] prometheus::Error),
    #[error("Prometheus deregistration error")]
    DeRegistrationError,
    #[error("UTF-8 conversion error: {0}")]
    Utf8ConversionError(#[from] std::string::FromUtf8Error),
    #[error("Error occurred while creating hyper server : {0}")]
    ServerError(#[from] hyper::Error),
    #[error("Error while loading the Certificate :{0}")]
    IoError(#[from] io::Error),
    #[error("Provide certificate path")]
    CertificatePathEmpty,
    #[error("Provide private key path")]
    PrivateKeyPathEmpty,
    #[error("Private Key is empty")]
    PrivateKeyEmpty,
    #[error("Error occurred while fetching cgroup stats")]
    FailedToFetchCGroupStatus,
}

trait MetricRegistrar {
    fn register(&self, metric: Box<dyn Collector>) -> Result<(), PrometheusFactoryError>;
    fn unregister(&self, metric: Box<dyn Collector>) -> Result<(), PrometheusFactoryError>;
    fn gather_metrics(&self) -> Vec<MetricFamily>;
    fn reset_registry(&mut self);
}

#[derive(Debug, Clone)]
pub struct PrometheusFactory {
    pub registry: Registry,
    pub bind_address: String,
    port: u16,
    pub base_metrics: HashMap<String, Counter>,
    pub private_key_path: String,
    pub certificate_path: String,
}

impl PrometheusFactory {
    pub fn new(
        bind_address: String,
        port: u16,
        include_base_metrics: bool,
        base_labels: HashMap<String, String>,
        private_key_path: String,
        certificate_path: String,
    ) -> Result<Self, PrometheusFactoryError> {
        let mut factory = Self {
            registry: Registry::new(),
            bind_address,
            port,
            base_metrics: HashMap::new(),
            private_key_path,
            certificate_path,
        };
        if include_base_metrics {
            Self::append_base_metrics(base_labels, &mut factory)?;
        }
        Ok(factory)
    }

    fn append_base_metrics(
        base_labels: HashMap<String, String>,
        factory: &mut PrometheusFactory,
    ) -> Result<(), PrometheusFactoryError> {
        let stats =
            CgroupStats::new().map_err(|_| PrometheusFactoryError::FailedToFetchCGroupStatus)?;
        factory.set_or_update_base_counter_metric(
            "cpu_total_usec",
            "CPU time used in usec total",
            base_labels.clone(),
            stats.cpu.cpu_total_usec as f64,
        )?;
        factory.set_or_update_base_counter_metric(
            "cpu_user_usec",
            "CPU time used for userspace in usec",
            base_labels.clone(),
            stats.cpu.cpu_user_usec as f64,
        )?;
        factory.set_or_update_base_counter_metric(
            "cpu_system_usec",
            "CPU time used for kernel in usec",
            base_labels.clone(),
            stats.cpu.cpu_system_usec as f64,
        )?;
        factory.set_or_update_base_counter_metric(
            "mem_anon_bytes",
            "Anonymous memory used in bytes",
            base_labels.clone(),
            stats.mem.mem_anon_bytes as f64,
        )?;
        factory.set_or_update_base_counter_metric(
            "mem_file_bytes",
            "File-backed memory used in bytes",
            base_labels.clone(),
            stats.mem.mem_file_bytes as f64,
        )?;
        factory.set_or_update_base_counter_metric(
            "mem_sock_bytes",
            "Socket memory used in bytes",
            base_labels.clone(),
            stats.mem.mem_sock_bytes as f64,
        )?;
        Ok(())
    }
    pub fn set_or_update_base_counter_metric(
        &mut self,
        name: &str,
        help: &str,
        labels: HashMap<String, String>,
        value: f64,
    ) -> Result<(), PrometheusFactoryError> {
        let metric = match self.base_metrics.get_mut(name) {
            Some(existing_metric) => existing_metric,
            None => {
                let counter = self.build_counter(name, help, labels)?;
                self.base_metrics.insert(name.to_string(), counter);
                self.base_metrics
                    .get_mut(name)
                    .ok_or_else(|| PrometheusFactoryError::FailedToFetchMetric(name.to_string()))?
            }
        };
        metric.reset();
        metric.inc_by(value);
        Ok(())
    }
}

impl MetricRegistrar for PrometheusFactory {
    fn register(&self, metric: Box<dyn Collector>) -> Result<(), PrometheusFactoryError> {
        self.registry
            .register(metric)
            .map_err(PrometheusFactoryError::RegistrationError)
    }

    fn unregister(&self, metric: Box<dyn Collector>) -> Result<(), PrometheusFactoryError> {
        self.registry
            .unregister(metric)
            .map_err(PrometheusFactoryError::RegistrationError)
    }

    fn gather_metrics(&self) -> Vec<MetricFamily> {
        self.registry.gather()
    }

    fn reset_registry(&mut self) {
        self.registry = Registry::new();
    }
}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}
// Load public certificate from file.
fn load_certs(filename: &str) -> Result<Vec<rustls::Certificate>, PrometheusFactoryError> {
    // Open certificate file.
    let certfile = fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = io::BufReader::new(certfile);

    // Load and return certificate.
    let certs = rustls_pemfile::certs(&mut reader)
        .map_err(|e| error(format!("failed to load certificate:{}", e)))?;
    Ok(certs.into_iter().map(rustls::Certificate).collect())
}

// Load private key from file.
fn load_private_key(filename: &str) -> Result<rustls::PrivateKey, PrometheusFactoryError> {
    // Open keyfile.
    let keyfile = fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = io::BufReader::new(keyfile);

    // Load and return a single private key.
    let keys = rustls_pemfile::rsa_private_keys(&mut reader)
        .map_err(|e| error(format!("failed to load private key :{}", e)))?;
    if keys.len() != 1 {
        return Err(PrometheusFactoryError::PrivateKeyEmpty);
    }

    Ok(rustls::PrivateKey(keys[0].clone()))
}
impl PrometheusFactory {
    pub fn build_counter(
        &self,
        name: &str,
        help: &str,
        labels: HashMap<String, String>,
    ) -> Result<Counter, PrometheusFactoryError> {
        let opts = Opts::new(name, help).const_labels(labels);
        Counter::with_opts(opts.clone())
            .map_err(PrometheusFactoryError::RegistrationError)
            .and_then(|counter| {
                self.register(Box::new(counter.clone()))?;
                Ok(counter)
            })
    }

    pub fn build_histogram(
        &self,
        name: &str,
        help: &str,
        labels: HashMap<String, String>,
    ) -> Result<Histogram, PrometheusFactoryError> {
        let histogram_opts = HistogramOpts::new(name, help).const_labels(labels);
        Histogram::with_opts(histogram_opts)
            .map_err(PrometheusFactoryError::RegistrationError)
            .and_then(|histogram| {
                self.register(Box::new(histogram.clone()))?;
                Ok(histogram)
            })
    }

    pub fn build_gauge(
        &self,
        name: &str,
        help: &str,
        labels: HashMap<String, String>,
    ) -> Result<Gauge, PrometheusFactoryError> {
        let opts = Opts::new(name, help).const_labels(labels);
        Gauge::with_opts(opts)
            .map_err(PrometheusFactoryError::RegistrationError)
            .and_then(|gauge| {
                self.register(Box::new(gauge.clone()))?;
                Ok(gauge)
            })
    }

    pub fn build_int_counter(
        &self,
        name: &str,
        help: &str,
        labels: HashMap<String, String>,
    ) -> Result<IntCounter, PrometheusFactoryError> {
        let opts = Opts::new(name, help).const_labels(labels);
        IntCounter::with_opts(opts.clone())
            .map_err(PrometheusFactoryError::RegistrationError)
            .and_then(|counter| {
                self.register(Box::new(counter.clone()))?;
                Ok(counter)
            })
    }

    pub fn build_int_gauge(
        &self,
        name: &str,
        help: &str,
        labels: HashMap<String, String>,
    ) -> Result<IntGauge, PrometheusFactoryError> {
        let opts = Opts::new(name, help).const_labels(labels);
        IntGauge::with_opts(opts.clone())
            .map_err(PrometheusFactoryError::RegistrationError)
            .and_then(|gauge| {
                self.register(Box::new(gauge.clone()))?;
                Ok(gauge)
            })
    }

    fn create_socket_address(
        bind_address: &str,
        port: u16,
    ) -> Result<SocketAddr, PrometheusFactoryError> {
        // Parse the bind address
        let ip_addr: IpAddr = bind_address
            .parse()
            .map_err(|_| PrometheusFactoryError::InvalidIpAddress(bind_address.to_string()))?;

        // Create a socket address based on the parsed IP address and port
        Ok(SocketAddr::new(ip_addr, port))
    }
    async fn handle_request(
        _req: Request<Body>,
        factory: PrometheusFactory,
    ) -> Result<Response<Body>, PrometheusFactoryError> {
        let response_body = factory.render_metrics()?;
        Ok(Response::new(Body::from(response_body)))
    }

    pub async fn serve(
        &self,
        mut reload_config: Receiver<()>,
    ) -> Result<(), PrometheusFactoryError> {
        {
            let (tx, mut rx) = tokio::sync::mpsc::channel(10);
            tokio::spawn(async move {
                let tx = tx.clone();
                while (reload_config.recv().await).is_some() {
                    info!("SIGHUP received, now reloading the server with new configuration");
                    tx.send(()).await.ok();
                }
            });
            loop {
                // tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                if self.certificate_path.is_empty() {
                    return Err(PrometheusFactoryError::CertificatePathEmpty);
                }
                if self.private_key_path.is_empty() {
                    return Err(PrometheusFactoryError::PrivateKeyPathEmpty);
                }
                // Load public certificate.
                let certs = load_certs(self.certificate_path.as_str())?;
                // Load private key.
                let key = load_private_key(self.private_key_path.as_str())?;
                let socket_addr = PrometheusFactory::create_socket_address(
                    self.bind_address.as_str(),
                    self.port,
                )?;
                let incoming = AddrIncoming::bind(&socket_addr)?;
                let acceptor = TlsAcceptor::builder()
                    .with_single_cert(certs, key)
                    .map_err(|e| error(format!("{}", e)))?
                    .with_all_versions_alpn()
                    .with_incoming(incoming);

                let make_svc = make_service_fn(move |_conn| {
                    let factory = self.clone();
                    async move {
                        Ok::<_, hyper::Error>(service_fn(move |req| {
                            let factory = factory.clone();
                            Self::handle_request(req, factory)
                        }))
                    }
                });
                let server = Server::builder(acceptor).serve(make_svc);
                info!("Exporter listening on http://{}", socket_addr);
                let mut reloading_requested = false;
                tokio::select! {
                    _ = rx.recv() => {
                        reloading_requested = true;
                    },
                    e = server => {
                            error!("server error: {:?}", e);
                    }
                }
                if reloading_requested {
                    info!("Received SIGHUP,reloading server with new certificate");
                    continue;
                }
            }
            Ok(())
        }
    }
}

impl RenderToPrometheus for PrometheusFactory {
    fn render_metrics(&self) -> Result<String, PrometheusFactoryError> {
        let encoder = TextEncoder::new();
        let mut buffer = vec![];
        encoder.encode(&self.gather_metrics(), &mut buffer)?;
        String::from_utf8(buffer).map_err(PrometheusFactoryError::Utf8ConversionError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::labels;
    #[test]
    fn test_reset_factory() {
        let mut factory = PrometheusFactory::new(
            String::from("127.0.0.1"),
            8080,
            false,
            HashMap::new(),
            "examples/sample.rsa".to_string(),
            "examples/sample.pem".to_string(),
        )
        .unwrap();
        let labels = labels! {
                "service".to_string() => "compute".to_string(),
                "source".to_string() => "versatus".to_string(),
        };
        let counter = factory
            .build_counter("counter", " counter metric", labels.clone())
            .unwrap();
        let gauge = factory
            .build_gauge("gauge", " gauge metric", labels.clone())
            .unwrap();
        let histogram = factory
            .build_histogram("histogram", " histogram metric", labels)
            .unwrap();
        counter.inc();
        gauge.set(12.0);
        gauge.inc();
        histogram.observe(7.0);
        assert_eq!(factory.gather_metrics().len(), 3);
        factory.reset_registry();
        assert_eq!(factory.gather_metrics().len(), 0);
    }
}
