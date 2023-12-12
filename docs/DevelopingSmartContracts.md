# Developing Smart Contracts On Versatus

## Overview

Whether you're new to development of smart contracts for blockchain, or if you have experience with traditional smart contract platforms, there will be some differences when developing smart contracts on Versatus. The catalysts for these differences have been:
1. Ease of development -- especially for those not already familiar with smart contract development
2. Language agnosticism -- we feel that you shouldn't need to learn a new language, compiler, IDE, debugger to develop smart contracts.
3. Security -- even if you're new to smart contract security
4. Scale -- by executing off-chain, more parallel compute jobs can be executed at once

This document aims to be a good starting point for anyone learning to develop smart contracts for Versatus, regardless of the language(s) and tools you intend to develop in.

## Compilation and Execution Target

In order to best-support smart contract development in a broad range of languages, Versatus smart contracts are compiled from source code into a binary execution format called [Web Assembly (WASM)](https://webassembly.org/). Web Assembly is a mature and standard technology that allows for secure and performant execution of code on a variety of hardware and software plaforms, regardless of the language used to develop the software.

You can find a brief article that describes why Versatus chose Web Assembly [here](https://incomplete.io/wasm/why-wasm-versatus/index.html).

Essentially what this means is that if your favourite programming language can be compiled down to Web Assembly, there is a very good chance that it can be used to write smart contracts on Versatus. If your language of choice isn't supported today, we'll include some notes below that enumerate the requirements for adding a new language in case you want to add your own.

## Language Standard Libraries

Web Assembly is designed to be very secure and to limit what an executing program is able to do. As a result, many things you might ordinarily expect to be able to do in a program written in your language of choice may be limited or disabled entirely. This includes things like file and network I/O and multi-threading.

However, when you consider a smart contract and how it is executed (either traditionally or on Versatus), the reasons for limiting these types of I/O become a little more obvious. A smart contract is executed across a large quorum of nodes on a network -- in some networks, this may be all of the nodes on the network. Now imagine if a smart contract were able to make a bunch of network connections outside of the smart contract environment. This might make it possible for a smart contract to be developed to target one or more services on the internet, and for that to create a Distributed Denial-Of-Service (DDOS) attack. This is just one example from the whole security attack surface.

In order to keep things simple and secure to develop, host and audit, we severely limit these kinds of operations for smart contracts. At least in the short term.

## Language Specifics

The Web Assembly tools for each language vary in quality and accessibility. For the most popular languages (we commissioned a formal survey), the tooling for WASM is generally pretty good. Within Versatus, we have spent time working with various WASM toolchains for various languages, and have implemented a thin SDK for each language to provide helper functionality for writing smart contracts quickly. Where possible, we have aimed to make this functionality fit well within the ecosystem of that language. For example, the Rust, Go and Javascript functionality is provided as language packages for each, so that they can be quickly pulled into new projects.

Each language has its own Github repository:

* [Rust](https://github.com/versatus/versatus-rust)
* [Javascript](https://github.com/versatus/versatus-javascript)
* [Go](https://github.com/versatus/versatus-go)
* [C++](https://github.com/versatus/versatus-cpp)
* [C](https://github.com/versatus/versatus-c)
* [Python](https://github.com/versatus/versatus-python)

## Off-Chain Testing

In order to keep things developer-friendly, Versatus provides its smart contract runtime for common development platforms as a way to execute and test smart contracts in isolation in a developer's workspace or any CI/CD pipeline.

> [!NOTE]
> mattgeddes@ to fill in the specific commands here as they're available.

## Adding New Language Support

Currently, the requirements on a given language for being supported as a smart contract development language on Versatus is very minimal. There are only two requirements:

1. That the language can be compiled down to a Web Assembly execution target, and
2. That the Web Assembly target supports the WASI systems interface. Behind the scenes, Versatus smart contracts are passed in all of their state as JSON on `stdin` and write all of their output on `stdout`.

Other than that, we use language-specific constructs such as _Abstract Classes_, _Traits_ or _Interfaces_ to templatise common smart contract patterns for developers. This isn't strictly required, but makes it easier for developers.

