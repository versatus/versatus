> Matt Geddes

We have external RPCs for wallets and apps to request things of the network. That's the vrrb_rpc crate.
We also need to have RPCs for internal communication, where internal is internal to a node operator. 
Essentially the network requests that will allow the protocol to tell the compute stack to run a compute job, 
or to tell the storage stack to pin a package or for the compute stack to retrieve a package from storage. 
Plus general status/capabilities stuff for use between services (detail below) and by things like the node operator UI.

We'll need some of the common plumbing stuff -- having it read the client/server config using the service_config crate, 
starting the listener in the protocol, compute and storage services, using that same config to work out which IP/port/cert/key 
to use for the client (eg versatus-compute client status) to have us be able to call RPCs from the command line for testing, 
debugging, etc, etc. And also to implement some actual RPC calls -- I'd suggest starting with a ServiceStatus RPC call that 
can be used to query any of our services and get a response, including the capabilities of that service. The capabilities 
stuff can be added to the platform  crate and included from there -- nothing super fancy for now.

1. reading the client/server config: does this mean have some function in a library that accepts a service_config::Config and
returns something?
2. starting the listener: is this like a forward command to the other agents, or is it a listener that lives within the
internal_rpc crate?
3. be able to call RPCs from the command line: is the internal_rpc a command line tool/binary product or is it a library
of code that gets called elsewhere?
