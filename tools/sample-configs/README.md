# Sample Configs

This directory contains sample configuration files that may be used as defaults or modified to suit a given deployment. Descriptions of each follow.

## services.json

This configuration file is used by the storage agent, the compute agent and (perhaps) the protocol service for defining RPC endpoints within a given operator's deployment. It is used, for example, for RPC clients to determine which address, port and secret key to use to communicate with specific services over RPC. It is also used by RPC services to know which RPC address and port to bind to and which secret key to use when authenticating incoming client requests.

The default file is suitable for deployments where all three (compute, protocol and storage) services are running on the same machine. All that is required is to change each of the pre-shared keys to a random string each.

