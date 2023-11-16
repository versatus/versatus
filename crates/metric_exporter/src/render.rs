use crate::metric_factory::PrometheusFactoryError;

pub trait RenderToPrometheus: std::fmt::Debug {
    fn render_metrics(&self) -> Result<String, PrometheusFactoryError>;
}
