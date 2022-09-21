## WORKFLOW-RPC

Part of the [WORKFLOW-RS](https://github.com/workflow-rs) application framework.

***

Platform-neutral RPC Client and Native RPC Server

[![Crates.io](https://img.shields.io/crates/l/workflow-rpc.svg?maxAge=2592000)](https://crates.io/crates/workflow-rpc)
[![Crates.io](https://img.shields.io/crates/v/workflow-rpc.svg?maxAge=2592000)](https://crates.io/crates/workflow-rpc)
![platform](https://img.shields.io/badge/platform-Native/client-informational)
![platform](https://img.shields.io/badge/platform-Native/server-informational)
![platform](https://img.shields.io/badge/platform-Web/client%20%28wasm32%29-informational)

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
