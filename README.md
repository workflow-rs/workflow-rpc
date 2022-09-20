## WORKFLOW-RPC

Part of the [WORKFLOW-RS](https://github.com/workflow-rs) application framework.

***

Platform-neutral RPC Client and Native RPC Server

Platforms supported: Native (client & server), WASM (browser: client)

## Features

RPC library based on top of WORKFLOW-WEBSOCKET that offers both synchronous and asynchronous Binary and JSON data relay over Workflow-WebSocket-based connections. 

The goal of this crate is to reduce boilerplate as much as possible
allowing remote function invocation using a single function with two generics `rpc.call<Req,Resp>()` where request and response must implement serlialization the respective serialization traits.

Binary RPC uses Borsh serialization and JSON RPC uses Serde serialization.

## Implementation status

- [x] Asynchronous Binary RPC Client
- [x] Asynchronous Binary RPC Server
- [ ] Asynchronous Binary RPC Server Notifications
- [ ] Synchronous JSON RPC Client
- [ ] Synchronous JSON RPC Server
- [ ] Synchronous RPC Server Notifications
