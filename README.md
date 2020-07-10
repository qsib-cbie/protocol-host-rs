# CLI for VR Actuator Manipulation

This repo contains a Rust CLI for manipulating VR Actuators. This includes a daemon for controlling adapters for communication over NFC or BLE. This also includes a client for submitting commands from the terminal. The client and server communicate over ZMQ.

## Dependencies

- Rust for building the binary. https://rustup.rs
- ZMQ for messaging. https://zeromq.org/download/
- libusb for reader manipuation. https://libusb.info

## Running

- Start the daemon in one session. `cargo run -- -vv start`
- Send commands in another session. `cargo run -- -vv command`
- See help for more info. `cargo run -- --help`