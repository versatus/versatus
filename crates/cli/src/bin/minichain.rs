use node::{self, Node};

#[tokio::main]
async fn main() {
    let mut node_0 = Node::start(&node::test_utils::create_mock_full_node_config())
        .await
        .unwrap();

    let mut node_1 = Node::start(&node::test_utils::create_mock_full_node_config())
        .await
        .unwrap();

    let mut node_2 = Node::start(&node::test_utils::create_mock_full_node_config())
        .await
        .unwrap();

    let mut node_3 = Node::start(&node::test_utils::create_mock_full_node_config())
        .await
        .unwrap();

    let mut node_4 = Node::start(&node::test_utils::create_mock_full_node_config())
        .await
        .unwrap();

    let mut node_5 = Node::start(&node::test_utils::create_mock_full_node_config())
        .await
        .unwrap();
    //
    // do soemthing in between
    //

    node_0.stop();
    node_1.stop();
    node_2.stop();
    node_3.stop();
    node_4.stop();
    node_5.stop();
}
