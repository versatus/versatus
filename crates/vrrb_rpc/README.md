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
# createTxn
# note: in order to actually create another tx, one must change the payload
# try iterating the timestamp
$ {"jsonrpc":"2.0","id":"1","method":"state_createTxn","params":[{"timestamp":1678756128,"sender_address":"0351615b78ae431509ccf19f3d55e19e07baac0a4d024b999ff1c4234207d4410a","sender_public_key":"031c0c705bee9901be2c221b71c490239b86d1518e1eeca9e9c0565f8da5e53797","receiver_address":"0351615b78ae431509ccf19f3d55e19e07baac0a4d024b999ff1c4234207d44106","token":{"name":"VRRB","symbol":"VRRB","decimals":18},"amount":0,"signature":"3045022100cfd569e53190fb9e01e6dfce8895049d953539c527862d818c2ac0dcf763bcf00220085bf1c74828121c21b0fb25621073408663891f9aca74c525421f963910b3ef","validators":{},"nonce":0,"receiver_farmer_id":null}]}
# sign
$ {"jsonrpc":"2.0","id":"1","method":"state_signTransaction","params":[{"timestamp":1678756128,"sender_address":"0351615b78ae431509ccf19f3d55e19e07baac0a4d024b999ff1c4234207d4410a","sender_public_key":"031c0c705bee9901be2c221b71c490239b86d1518e1eeca9e9c0565f8da5e53797","receiver_address":"0351615b78ae431509ccf19f3d55e19e07baac0a4d024b999ff1c4234207d44106","token":{"name":"VRRB","symbol":"VRRB","decimals":18},"amount":0,"nonce":0, "private_key":"ba6ec9325d42dfde5ef2f24ea9f58dd23147e8604146c41fa8abc809c0ba3e21"}]}
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
