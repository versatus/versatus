# versatus-wasm Command Line Reference

## Name

`versatus-wasm` -- Versatus Web Assembly Runtime

## Synopsis

```shell
versatus-wasm OPTION... [publish|describe|validate|execute]...
```

## Description

`versatus-wasm` is the Versatus Web Assembly Runtime. It is used to execute smart contracts on the Versatus Network, and is also available as a stanadalone tool for developers to develop, test and publish their smart contracts to the Versatus Network.

## Subcommmands

A variety of functionality may be called in `versatus-wasm` through various subcommands, including:

* `describe`
* `validate`
* `execute`
* `publish`

These are described in detail below.

### `describe`

Given the path to a Web Assembly file, show some basic information about it. It supports the following options:

* `-h`, `--help` -- Show usage help text for the describe subcommand.
* `-w`, `--wasm` `<FILE>` -- The path to the WASM object file to load and describe.

For example:

```shell
versatus-wasm describe --wasm ./contract.wasm
```

### `validate`

Given the path to a Web Assembly file, try to validate whether it will run on the Versatus Network.

* `-h`, `--help` -- Show usage help text for the validate subcommand.
* `-w`, `--wasm` `<FILE>` -- The path to the WASM object file to load and describe.

For example:

```shell
versatus-wasm validate --wasm ./contract.wasm
```

### `publish`

Given a Web Assembly Smart Contract for the Versatus Network, along with some optional metadata, publish the contract as a package to the Versatus Network.

* `-a`, `--author <AUTHOR>` -- The author of the package. May be an empty string.
* `-h`, `--help` -- Show usage help text for the validate subcommand.
* `-n`, `--name <NAME>` -- The name of the package to create. May be an empty string.
* `-v`, `--version <VERSION>` -- The version of the package.
* `-w`, `--wasm <FILE>` -- A.The path to the WASM object file to package and publish

Metadata fields such as `author` and `name` are a convenience and not relied upon anywhere in the Versatus network.

For example:

```shell
versatus-wasm publish \
    --wasm ./contract.wasm \
    --author "Versatus Developer" \
    --version 1 \
    --name "ERC20 token for compute units"
```

### `execute`

Given a Web Assembly Smart Contract for the Versatus Network, and a JSON file representing the input to the contract, execute the smart contract and display its output.

* `-e`, `--env <KEY=VALUE>` -- An environment variable to pass to the running WASM module. May be used multiple times.
* `-h`, `--help` -- Show usage help text for the execute subcommand.
* `-j`, `--json` -- The path to JSON file to become input to the running WASM module.
* `-l`, `--meter-limit` -- The credit limit for WASM execution by the contract.
* `-w`, `--wasm <FILE>` -- The path the WASM object to load and execute.

For example:
```shell
versatus-wasm execute --wasm ./contract.wasm --json ./inputs.json
```
