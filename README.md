<p align="center">
    <a href="https://discord.gg/versatus" alt="Discord">
        <img src="https://img.shields.io/discord/1034112774789414963.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2&style=for-the-badge" />
    </a>
    <a href="https://twitter.com/VersatusLabs?s=20" alt="Twitter">
        <img src="https://img.shields.io/twitter/follow/VersatusLabs?style=for-the-badge&logo=twitter&logoColor=white&labelColor=1DA1F2&color=1DA1F2" />
    </a>
</p>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://github.com/versatus/brand-assets/blob/33cf3981d13f439a43ddfde5966a8a5fd58ff5ce/logo/versatus_logo_white.png">
  <img alt="VRRB Logo" src="https://github.com/versatus/brand-assets/blob/33cf3981d13f439a43ddfde5966a8a5fd58ff5ce/logo/versatus_logo_white.png">
</picture>

Versatus is a fast, scalable Layer 1 blockchain truly designed for mass adoption.
Versatus is powered by a novel consensus mechanism called Proof-of-Claim, which
features dual-scalability for record-breaking TPS and fast time to finality.
Versatus offers a flexible, composable and safe, (programming) language agnostic
compute platform powered by unikernel containers. The Versatus compute platform
takes web3 beyond the paradigm of ownership, tokenization and event
triggered transfers, offering a decentralized general execution environment
that can deliver on the promise of a decentralized internet and financial

Versatus provides the most flexible developer experience available on any
smart contract platform, allowing developers to build in any language they
choose. Beyond language agnosticism, Versatus programs are secure by design,
allowing developers to spend more time thinking about building core
business logic and design. This provides developers with shorter roadmaps,
lower audit costs, leads to fewer bugs, less maintenance, and ultimately,
will lead to better end user experiences.
<br>

<hr>
<br>

<div align="center">
  <img src="https://github.com/versatus/brand-assets/blob/33cf3981d13f439a43ddfde5966a8a5fd58ff5ce/memes/fexible-text.gif" alt="nowthatsflexible.gif">
</div>
<br>
<hr>
<br>

Versatus is a permissionless network, be they operators, developers, users or
some combination thereof, Versatus is decentralized, censorship resistant and
accessible to anyone with an internet connection.

<br>
<hr>

Scroll to the bottom for information on how to start a node.

### High Level Roadmap

This is extremely high level, for each **Epic** there are multiple features
and under each feature there are many stories and tasks

_Items that are more than 50% complete are marked with :construction: while
items that are less than 50% complete are marked as :x:, all items marked with
:white_check_mark: are complete, tested, and integrated into the node runtime_

:link: : Alphanet
:signal_strength: Betanet
:computer: Devnet

| **Epic**                  | _Description_                                                                                                                                                                                               | State              | Network           |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------ | ----------------- |
| Network                   | P2P Network enabling communication between network participants                                                                                                                                             | :white_check_mark: | :link:            |
| Election                  | Proof of Claim Algorithm Implementation and Integration                                                                                                                                                     | :white_check_mark: | :link:            |
| Genesis Quorum Protocol   | Formation of the first quorums at Genesis event                                                                                                                                                             | ✅     | :link:            |
| Key Generator             | Protocol to generate Dealerless Distributed Keypairs for validator nodes, and ECDSA keypairs for all nodes                                                                                                  | :white_check_mark: | :link:            |
| State Store               | Left-Right Wrapped Accounts Database and State Trie                                                                                                                                                         | ✅                 | :link:            |
| Mempool                   | Left-Right Wrapped Pending Transaction Store                                                                                                                                                                | :white_check_mark: | :link:            |
| Validator Unit            | Left-Right enabled transaction validation protocol                                                                                                                                                          | :white_check_mark: | :link:            |
| Farmer-Harvester          | Farmer-Havester Quorum model for secure parallel execution and validation of transactions                                                                                                                   | ✅                 | :link:            |
| DAG                       | Rounds based Directed Acyclic Graph to append blocks to                                                                                                                                                     | :white_check_mark: | :link:            |
| Miner Unit                | Protocol for consolidating proposal blocks produced by miners into a single point of reference signifying the end of a round and finality of transactions (once certified)                                  | :white_check_mark: | :link:            |
| Scheduler                 | Decentralized task buffer and allocator to maximize efficiency of Farmer Quorum nodes                                                                                                                       | :white_check_mark: | :link:            |
| Block Production          | Enables harvesters to produce conflict minimized, extractable value maximized proposal blocks to be appended to the DAG                                                                                     | :white_check_mark: | :link:            |
| Node CLI                  | Provides an interface for operators to spin up a Versatus node                                                                                                                                                  | :white_check_mark: | :link:            |
| Wallet CLI                | Provides an interface for users to interact with the Versatus network                                                                                                                                           | 🚧     | :link:            |
| Token Emission Protocol   | Ensures that proper number of tokens each block and epoch are produced                                                                                                                                      | :white_check_mark: | :signal_strength: |
| Fee Model                 | Provides economic incentives to operators beyond emission subsidy, provides token burning to limit inflation, and economic incentives to maintain speed at scale                                            | :x:                | :signal_strength: |
| Reputation Tracking       | Tracks the reputation of nodes, and the message credits, to align incentives, reduce malicious behavior and allow for dynamic stake calculation to prevent accumulation and centralization of staking nodes | :construction:     | :signal_strength: |
| Dynamic Stake Calculator  | Protocol to calculate minimum required stake of nodes in the network in order for the given nodes to become eligible as validators                                                                          | :x:                | :signal_strength: |
| Block Indexer             | Indexes, sequences and stores blocks for display in UIs that need access to block and transaction data                                                                                                      | :construction:     | :signal_strength: |
| Block Explorer            | Provides a web based user interface for scanning blocks, tracking transactions, etc.                                                                                                                        | :construction:     | :signal_strength: |
| Node Metrics              | Tracks the performance of a given node, cluster of nodes, and/or all nodes in the network                                                                                                                   | :x:                | :signal_strength: |
| Rent Model                | Provides economic incentives to developers to build small, modular programs that do one thing well, and link them together by returning commands to the orchestration network.                              | :x:                | :computer:        |
| Whistleblower Protocol    | Protocol for reporting malicious behavior, and initiating a stake slashing vote                                                                                                                             | :x:                | :computer:        |
| Wallet GUI                | Provides a user interface for interacting with Versatus network                                                                                                                                                 | :x:                | :computer:        |
| Unikernel Compute Runtime | Enables programming language agnostic compute in the Versatus network                                                                                                                                           | :x:                | :computer:        |

### Starting a Node

In order to start a node, run `cargo run`
Running `cargo run -- -help` will display available cli flags for node configuration and management.

**The above builds and runs a Versatus node in `debug` mode** to run in optimized
release mode you must first build the release target using the following command:

```
git clone https://github.com/versatus/versatus
cd /path/to/cloned/repo
cargo build --release
```

This will produce a target file (and directory if this is your first time
running `cargo run` or `cargo build` in this repo.

Then, to display the available CLI flags, you can run:

```
cd target/release
./versatus -help
```
