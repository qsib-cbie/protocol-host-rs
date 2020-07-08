# CLI for VR Actuator Manipulation

This repo contains a Rust CLI for manipulating VR Actuators. This includes a daemon for controlling adapters for communication over NFC or BLE. This also includes a client for submitting commands from the terminal. The client and server communicate over ZMQ.

## Dependencies

- Rust for building the binary. https://rustup.rs
- ZMQ for messaging. https://zeromq.org/download/

## Running

- Start the daemon. `cargo run -- start`
- Send commands. `cargo run -- command 01234ABCDEF`
- Shutdown the daemon. `cargo run -- stop`
- See help for more info. `cargo run -- --help`