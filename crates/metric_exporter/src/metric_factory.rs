use crate::render::RenderToPrometheus;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use prometheus::proto::MetricFamily;
use prometheus::{
    core::Collector, Counter, Encoder, Gauge, Histogram, HistogramOpts, IntCounter, IntGauge, Opts,
    Registry, TextEncoder,
};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use thiserror::Error;
use platform::platform_stats::CgroupStats;

#[derive(Debug, Error)]
pub enum PrometheusFactoryError {
    #[error("Prometheus registration error: {0}")]
    RegistrationError(#[from] prometheus::Error),
    #[error("Prometheus deregistration error")]
    DeRegistrationError,
    #[error("UTF-8 conversion error: {0}")]
    Utf8ConversionError(#[from] std::string::FromUtf8Error),
    #[error("Error occurred while creating hyper server : {0}")]
    ServerError(#[from] hyper::Error),
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
    port: u16,
    pub base_metrics: HashMap<String, Counter>,
}

impl PrometheusFactory {
    pub fn new(
        port: u16,
        include_base_metrics: bool,
        base_labels: HashMap<String, String>,
    ) -> Self {
        let mut factory = Self {
            registry: Registry::new(),
            port,
            base_metrics: HashMap::new(),
        };
        if include_base_metrics {
            Self::append_base_metrics(base_labels, &mut factory);
        }
        factory
    }

    fn append_base_metrics(base_labels: HashMap<String, String>, factory: &mut PrometheusFactory) {
        let stats = CgroupStats::new().unwrap();
        factory.set_or_update_base_counter_metric("cpu_total_usec", "CPU time used in usec total", base_labels.clone(), stats.cpu.cpu_total_usec as f64);
        factory.set_or_update_base_counter_metric("cpu_user_usec", "CPU time used for userspace in usec", base_labels.clone(), stats.cpu.cpu_user_usec as f64);
        factory.set_or_update_base_counter_metric("cpu_system_usec", "CPU time used for kernel in usec", base_labels.clone(), stats.cpu.cpu_system_usec as f64);
        factory.set_or_update_base_counter_metric("mem_anon_bytes", "Anonymous memory used in bytes", base_labels.clone(), stats.mem.mem_anon_bytes as f64);
        factory.set_or_update_base_counter_metric("mem_file_bytes", "File-backed memory used in bytes", base_labels.clone(), stats.mem.mem_file_bytes as f64);
        factory.set_or_update_base_counter_metric("mem_sock_bytes", "Socket memory used in bytes", base_labels.clone(), stats.mem.mem_sock_bytes as f64);
    }
    pub fn set_or_update_base_counter_metric(
        &mut self,
        name: &str,
        help: &str,
        labels: HashMap<String, String>,
        value: f64,
    ) {
        let metric = match self.base_metrics.get_mut(name) {
            Some(existing_metric) => existing_metric,
            None => {
                let counter = self.build_counter(name, help, labels).unwrap();
                self.base_metrics.insert(name.to_string(), counter);
                self.base_metrics.get_mut(name).unwrap()
            }
        };
        metric.reset();
        metric.inc_by(value);
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
    async fn handle_request(
        _req: Request<Body>,
        factory: PrometheusFactory,
    ) -> Result<Response<Body>, PrometheusFactoryError> {
        let response_body = factory.render_metrics()?;
        Ok(Response::new(Body::from(response_body)))
    }
    pub async fn serve(&self) -> Result<(), PrometheusFactoryError> {
        {
            let make_svc = make_service_fn(move |_conn| {
                let factory = self.clone();
                async move {
                    Ok::<_, hyper::Error>(service_fn(move |req| {
                        let factory = factory.clone();
                        Self::handle_request(req, factory)
                    }))
                }
            });

            let socket_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, self.port));
            let server = Server::try_bind(&socket_addr)
                .map_err(PrometheusFactoryError::ServerError)?
                .serve(make_svc);

            log::info!("Exporter listening on http://{}", socket_addr);

            if let Err(e) = server.await {
                eprintln!("server error: {}", e);
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
        let mut factory = PrometheusFactory::new(8080, false, HashMap::new());
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
