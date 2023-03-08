# VRRB
## Versatile Ranged Reward Blockchain

VRRB is a Web-Scale Blockchain with an innovative low energy consumption consensus algorithm called Proof of Claim 
and a commodity production inspired monetary policy.

VRRB is currently a WIP scheduled to begin alpha testing on the MVP by end of year. If you are interested in contributing to the
project please reach out to the owners.

Node Requirements
    A minimum of 10,000 staked VRRB coins are required for Masternode eligibility, though this minimum may be higher depending on a node's reputation score.
    

### Starting a Node
    In order to start a node, run `cargo run`
    Running `cargo run -- -help` will display available cli flags for node configuration and management.

### RPC Testing

Using 2 terminal instances, run node and consume RPC endpoints.

```bash
# In terminal instance 1, navigate to root of repo
# start node
$ cargo run node run
```

In 2nd terminal using wscat

```bash
# Install wscat globally
# Info: https://www.npmjs.com/package/wscat
$ npm install -g wscat
# In terminal instance 2, navigate to root of repo
# Start ws connection
$ wscat -c ws://127.0.0.1:9293
# Get full state
$ {"jsonrpc":"2.0","id":"1","method":"state_getFullState","params":[]}
# Get full mempool
$ {"jsonrpc":"2.0","id":"1","method":"state_getFullMempool","params":[]}
# Get node type
$ {"jsonrpc":"2.0","id":"1","method":"state_getNodeType","params":[]}
# Get Account
# TODO: understand how to retrieve pub key
$ {"jsonrpc":"2.0","id":"1","method":"state_getAccount","params":["{pub_key}"]}
# TODO: document other RPC endpoints 
```

In 2nd terminal using curl

```bash
# In terminal instance 2, navigate to root of repo
# Get node type
$ curl -X POST -i -H "Accept: application/json" -H "Content-Type: application/json" -d '{"params": {},"jsonrpc": "2.0", "id": "1","method":"state_getNodeType"}' http://localhost:9293
# TODO: document other RPC endpoints 
```