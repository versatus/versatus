use std::{path::PathBuf, rc::Rc, sync::Arc};

use node::core::NodeType;
use runtime::{Runtime, RuntimeModuleState, RuntimeOpts};
use telemetry::TelemetrySubscriber;
use tokio::sync::oneshot;

#[tokio::test]
async fn node_runtime_starts_and_stops() {
    let (ctrl_tx, ctrl_rx) = oneshot::channel();

    let rt_opts = RuntimeOpts {
        node_type: NodeType::Full,
        data_dir: PathBuf::from("/tmp/vrrb"),
        node_idx: 100,
    };

    let mut node_rt = Runtime::new(ctrl_rx);
    assert_eq!(node_rt.status(), RuntimeModuleState::Stopped);

    let handle = tokio::spawn(async move {
        node_rt.start(rt_opts).await.unwrap();
        assert_eq!(node_rt.status(), RuntimeModuleState::Stopped);
    });

    ctrl_tx.send(primitives::StopSignal).unwrap();

    handle.await.unwrap();
}
