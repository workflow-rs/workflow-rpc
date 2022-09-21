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

RPC library built on top of [workflow-websocket](https://crates.io/crates/workflow-websocket) crate that offers both asynchronous Binary data relay over WebSocket connections. Synchronous & Asynchronous JSON RPC connections are planned for future releases.

The goal of this crate is to reduce boilerplate as much as possible allowing remote function invocation using a single function with two generics `rpc.call<Req,Resp>()` where request and response must implement serlialization using the respective serialization traits.

Binary RPC uses [Borsh](https://crates.io/crates/borsh) and JSON RPC uses [Serde](https://crates.io/crates/serde) serializers.

## Implementation status

- [x] Asynchronous Binary RPC Client
- [x] Asynchronous Binary RPC Server
- [ ] Asynchronous Binary RPC Server Notifications
- [ ] Synchronous JSON RPC Client
- [ ] Synchronous JSON RPC Server
- [ ] Synchronous RPC Server Notifications
