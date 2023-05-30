# Getting Started with VRRB

- [Getting Started with VRRB](#getting-started-with-vrrb)
- [Pre-requisites](#pre-requisites)
- [Clone the Github Repository](#clone-the-github-repository)
- [Compile the VRRB Node](#compile-the-vrrb-node)
- [Running the Node](#running-the-node)
  - [Node Arguments](#node-arguments)
  - [Running in Docker](#running-in-docker)

# Pre-requisites

In order to run the VRRB node, you will need to have the following installed on your machine:

- GIT (https://git-scm.com/downloads)
- Rust & Cargo (https://www.rust-lang.org/tools/install)
- GNU Make (https://www.gnu.org/software/make/)
- CLang (https://clang.llvm.org/)

# Clone the Github Repository

The VRRB code is hosted on Github.  Currently, we are not providing any pre-compiled releases so you will need to compile the code manually.

Clone the repository with the following command: 

```
# Clone the Repository
git clone https://github.com/vrrb-io/vrrb.git vrrb

# Change into the working directory
cd vrrb

# See the Makefile commands available
make 
```

# Compile the VRRB Node

The easiest way to compile the VRRB Node is using the Makefile.  The Makefile will compile the code and place the binary in the `target/release` directory.

```
# Compile the VRRB Node
make build
```

# Running the Node

After compiling, you can run the VRRB Node in development mode.  

```
# Call the `run` command and ensure the node can be started smoothly
cargo run -- node run --help
```

## Node Arguments

* `--bootstrap <bootstrap>`
    * Enables or disables the node's bootstrap mode.
    * Default: `false`

* `--bootstrap-node-addresses <BOOTSTRAP_NODE_ADDRESSES>`
    * A comma-separated list of bootstrap node addresses to connect to during startup.

* `-d, --detached`
    * Start the node as a background process.

* `--data-dir <DATA_DIR>`
    * Sets the directory for storing node data.
    * Default: `.vrrb`

* `--db-path <DB_PATH>`
    * Sets the path for the node's database.
    * Default: `.vrrb/node/db`

* `--debug-config`
    * Shows debugging config information.

* `--disable-networking`
    * Disables networking capabilities of the node.

* `-h, --help`
    * Prints help information.

* `--http-api-address <HTTP_API_ADDRESS>`
    * Sets the HTTP API server address.
    * Default: `127.0.0.1:0`

* `--http-api-title <HTTP_API_TITLE>`
    * Sets the title of the API shown on Swagger docs.
    * Default: `"Node RPC API"`

* `--http-api-version <HTTP_API_VERSION>`
    * Sets the API version shown in Swagger docs.
    * Default: `1.0.0`

* `-i, --id <ID>`
    * Specifies the node's unique identifier.

* `--idx <IDX>`
    * Specifies an additional node index.
    * Deprecated

* `--jsonrpc-api-address <JSONRPC_API_ADDRESS>`
    * Sets the JSON-RPC API server address.
    * Default: `127.0.0.1:9293`

* `--raptorq-gossip-address <RAPTORQ_GOSSIP_ADDRESS>`
    * Sets the address for RaptorQ gossip protocol communication.
    * Default: `127.0.0.1:0`

* `--rendezvous-local-address <RENDEZVOUS_LOCAL_ADDRESS>`
    * Sets the local address for rendezvous server communication.
    * Default: `127.0.0.1:0`

* `--rendezvous-server-address <RENDEZVOUS_SERVER_ADDRESS>`
    * Sets the address for connecting to the rendezvous server.
    * Default: `127.0.0.1:0`

* `-t, --node-type <NODE_TYPE>`
    * Defines the type of node created by this program. Valid options are `full`, `validator`, and `light`.
    * Default: `full`

* `--udp-gossip-address <UDP_GOSSIP_ADDRESS>`
    * Sets the address for UDP gossip protocol communication.
    * Default: `127.0.0.1:0`


## Running in Docker

```
# Build the Docker Image
docker build --no-cache -f Dockerfile -t vrrb . 

# Run the Docker Image
docker run --rm -it -p 8080:8080 -p 9293:9292 vrrb
```
