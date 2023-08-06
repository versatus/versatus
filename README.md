<picture>
  <source media="(prefers-color-scheme: dark)" srcset=https://github.com/vrrb-io/brand-assets/raw/main/logo/vrrb-logo-final1.png?raw=true">
  <!-- <img alt="VRRB Logo" src="https://github.com/vrrb-io/brand-assets/raw/main/logo/vrrb-logo-final2.png?raw=true"> -->
  <img alt="VRRB Logo" src="assets/logo_white.png">
</picture>

VRRB is a fast, scalable Layer 1 blockchain truly designed for mass adoption. 
VRRB is powered by a novel consensus mechanism called Proof-of-Claim, which 
features dual-scalability for record-breaking TPS and fast time to finality. 
VRRB offers a flexible, composable and safe, (programming) language agnostic 
compute platform powered by unikernel containers. The VRRB compute platform 
takes web3 beyond the paradigm of ownership, tokenization and event 
triggered transfers, offering a decentralized general execution environment 
that can deliver on the promise of a decentralized internet and financial

VRRB provides the most flexible developer experience available on any 
smart contract platform, allowing developers to build in any language they 
choose. Beyond language agnosticism, VRRB programs are secure by design,
allowing developers to spend more time thinking about building core 
business logic and design. This provides developers with shorter roadmaps,
lower audit costs, leads to fewer bugs, less maintenance, and ultimately,
will lead to better end user experiences.
<br>
<hr>
<br>

<div align="center">
  <!-- <img src="https://drive.google.com/uc?export=view&id=1Nx-DUr8e9ueRrgJ80dickT9S_XSztoAV" alt="nowthatsflexible.gif"> -->
  <img src="assets/flexible-text.gif" alt="nowthatsflexible.gif">
</div>
<br>
<hr>
<br>

VRRB is a permissionless network, be they operators, developers, users or 
some combination thereof, VRRB is decentralized, censorship resistant and 
accessible to anyone with an internet connection.

<br>
<hr>

Scroll to the bottom for information on how to start a node.

### High Level Roadmap

This is extremely high level, for each __Epic__ there are multiple features
and under each feature there are many stories and tasks

_Items that are more than 50% complete are marked with :construction: while 
items that are less than 50% complete are marked as :x:, all items marked with
:white_check_mark: are complete, tested, and integrated into the node runtime_

:link: : Alphanet
:signal_strength: Betanet
:computer: Devnet

| __Epic__   | _Description_ | State | Network |
|------------|---------------|-------|---------|
| Network | P2P Network enabling communication between network participants | :white_check_mark: | :link: |   
| Election | Proof of Claim Algorithm Implementation and Integration | :white_check_mark: | :link: |
| Genesis Quorum Protocol | Formation of the first quorums at Genesis event | :construction: | :link: |
| Key Generator | Protocol to generate Dealerless Distributed Keypairs for validator nodes, and ECDSA keypairs for all nodes | :white_check_mark: | :link: |
| State Store | Left-Right Wrapped Accounts Database and State Trie | ✅ | :link: |  
| Mempool | Left-Right Wrapped Pending Transaction Store | :white_check_mark: | :link: | 
| Validator Unit | Left-Right enabled transaction validation protocol | :white_check_mark: | :link: | 
| Farmer-Harvester | Farmer-Havester Quorum model for secure parallel execution and validation of transactions | ✅ | :link: |
| DAG | Rounds based Directed Acyclic Graph to append blocks to | :white_check_mark: | :link: | 
| Miner Unit | Protocol for consolidating proposal blocks produced by miners into a single point of reference signifying the end of a round and finality of transactions (once certified)| :white_check_mark: | :link: | 
| Scheduler | Decentralized task buffer and allocator to maximize efficiency of Farmer Quorum nodes | :white_check_mark: | :link: | 
| Block Production | Enables harvesters to produce conflict minimized, extractable value maximized proposal blocks to be appended to the DAG | :white_check_mark: | :link: |
| Node CLI | Provides an interface for operators to spin up a VRRB node | :white_check_mark: | :link: | 
| Wallet CLI | Provides an interface for users to interact with the VRRB network| :construction: | :link: | 
| Token Emission Protocol | Ensures that proper number of tokens each block and epoch are produced | :white_check_mark: | :signal_strength: |
| Fee Model | Provides economic incentives to operators beyond emission subsidy, provides token burning to limit inflation, and economic incentives to maintain speed at scale | :x: | :signal_strength: |
| Reputation Tracking | Tracks the reputation of nodes, and the message credits, to align incentives, reduce malicious behavior and allow for dynamic stake calculation to prevent accumulation and centralization of staking nodes| :construction: | :signal_strength: | 
| Dynamic Stake Calculator | Protocol to calculate minimum required stake of nodes in the network in order for the given nodes to become eligible as validators| :x: | :signal_strength: | 
| Block Indexer | Indexes, sequences and stores blocks for display in UIs that need access to block and transaction data | :construction: | :signal_strength: | 
| Block Explorer | Provides a web based user interface for scanning blocks, tracking transactions, etc. | :construction: | :signal_strength: | 
| Node Metrics | Tracks the performance of a given node, cluster of nodes, and/or all nodes in the network | :x: | :signal_strength: | 
| Rent Model | Provides economic incentives to developers to build small, modular programs that do one thing well, and link them together by returning commands to the orchestration network. | :x: | :computer: |
| Whistleblower Protocol | Protocol for reporting malicious behavior, and initiating a stake slashing vote | :x: | :computer: | 
| Wallet GUI | Provides a user interface for interacting with VRRB network | :x: | :computer: | 
| Unikernel Compute Runtime | Enables programming language agnostic compute in the VRRB network | :x: | :computer: | 

### Starting a Node

In order to start a node, run `cargo run`
Running `cargo run -- -help` will display available cli flags for node configuration and management.

__The above builds and runs a VRRB node in `debug` mode__ to run in optimized
release mode you must first build the release target using the following command:
```
git clone https://github.com/vrrb-io/vrrb
cd /path/to/cloned/repo
cargo build --release 
```

This will produce a target file (and directory if this is your first time 
running `cargo run` or `cargo build` in this repo.

Then, to display the available CLI flags, you can run:

```
cd target/release
./vrrb -help
```
