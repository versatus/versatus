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
# Get mempool length
$ {"jsonrpc":"2.0","id":"1","method":"state_getFullMempoolTxnCount","params":[]}
# Get full mempool
$ {"jsonrpc":"2.0","id":"1","method":"state_getFullMempool","params":[]}
# get mempool digests
$ {"jsonrpc":"2.0","id":"1","method":"state_getFullMempoolDigests","params":[]}
# Get node type
$ {"jsonrpc":"2.0","id":"1","method":"state_getNodeType","params":[]}
# Get Account
# TODO: understand how to retrieve pub key
$ {"jsonrpc":"2.0","id":"1","method":"state_getAccount","params":["{pub_key}"]}
# TODO: document other RPC endpoints 
```

In a 3rd terminal, one can create transactions against the node using the vrrb cli

```bash
# in root of vrrb
# create some accounts
$ cargo run wallet new --alias {int}
# take note of wallet public key printed in the vrrb node
# create transaction between accounts
$ cargo run wallet transfer --from 2 --to 0351615b78ae431509ccf19f3d55e19e07baac0a4d024b999ff1c4234207d4410a --amount 0
# note: accounts persist from txns in the mempool do not
# note: doesn't seem that the --to value matters, nonces aren't incrementing
```

In 2nd terminal using curl

```bash
# In terminal instance 2, navigate to root of repo
# Get node type
$ curl -X POST -i -H "Accept: application/json" -H "Content-Type: application/json" -d '{"params": {},"jsonrpc": "2.0", "id": "1","method":"state_getNodeType"}' http://localhost:9293
# TODO: document other RPC endpoints 
```
