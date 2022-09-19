use std::{path::PathBuf, rc::Rc, sync::Arc};

use commands::command::Command;
use futures::Future;
use node::core::NodeType;
use runtime::{Runtime, RuntimeModuleState, RuntimeOpts};
use telemetry::TelemetrySubscriber;

#[tokio::test]
async fn node_runtime_starts_and_stops() {
    let (ctrl_tx, ctrl_rx) = tokio::sync::mpsc::unbounded_channel::<Command>();

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

    ctrl_tx.send(Command::Stop).unwrap();

    handle.await.unwrap();
}

#[tokio::test]
async fn multiple_node_runtime_starts_and_stops() {
    let mut senders = Vec::<tokio::sync::mpsc::UnboundedSender<Command>>::new();
    let mut handles = Vec::<tokio::task::JoinHandle<()>>::new();

    (0..8).for_each(|i|{
        let (ctrl_tx, ctrl_rx) = tokio::sync::mpsc::unbounded_channel::<Command>();

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

        senders.push(ctrl_tx);
        handles.push(handle);
    });

    (0..8).for_each(|i|{
        senders[i].send(Command::Stop).unwrap();
    });

    for handle in handles{
        handle.await.unwrap();
    }

}

