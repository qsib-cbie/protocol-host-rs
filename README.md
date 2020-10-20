# CLI for VR Actuator Manipulation

This repo contains a Rust CLI for manipulating VR Actuators. This includes a daemon for controlling adapters for communication over NFC or BLE. This also includes a client for submitting commands from the terminal. The client and server communicate over ZMQ.

## Dependencies

- Rust for building the binary. https://rustup.rs
- ZMQ for messaging. https://zeromq.org/download/
- libusb for reader manipuation. https://libusb.info

## Running

### Antenna Host

The Feig Reader should be attached by USB to the host where we `start` a server. The command to start the antenna host may look like:
 ```bash
cargo run --release -- -vv start --protocol tcp --hostname ubuntu20 --port 6001
 ```
 The antenna host accepts commands by listening on a DEALER socket acting as an async REP socket. The socket should be connected to a DEALER from a ROUTER:DEALER or another socket that will prepend the multipart message with an unused id (optionally empty).

### Remote Client

 The client that wishes to manipulate the host may do so via proxy. Using the cli, `command` the antenna host using a list of commands to execute see the `command --help` for more options. The client acts as a REQ socket but connects as a DEALER for optional async messaging.

 Clients are expected to wait for the response of the previous command but are not required to. The server will always process the commands in serial order, so the commands will still end up being queued if delayed. The following runs a list of commands that will change the radio frequency power to low power mode on the Feig Reader.
 ```bash
cargo run --release -- -vv command --protocol tcp --hostname ubuntu20 --port 6000 commands/set-power-0.txt
 ```


### ZMQ Proxy

There is expected to be a ZMQ proxy running on a public IP host for routing traffic. Following the REQ-REP broker example (rrbroker), there is a proxy running on a virtual machine (aliased as ubuntu20 in my `/etc/hosts`). The example `ubuntu20` virtual machine is running the ROUTER on port 6000 and DEALER on port 6001.

```rust
impl RRBroker {
    pub fn proxy(front_endpoint: &str, back_endpoint: &str) -> Result<(), CliError> {
        log::info!("Starting proxy for {} and {}", front_endpoint, back_endpoint);

        let ctx = zmq::Context::new();
        let frontend = ctx.socket(zmq::ROUTER)?;
        let backend = ctx.socket(zmq::DEALER)?;

        frontend.bind(front_endpoint)?;
        backend.bind(back_endpoint)?;

        log::info!("Bound and beginning proxy as ROUTER:DEALER");

        zmq::proxy(&frontend, &backend)?; // Never returns
    }
}
```

To run your own router, clone and build the rust CLI at https://github.com/trueb2/zmq-cli-rs.git. You can run a proxy on your localhost with

```bash
cd ~
git clone https://github.com/trueb2/zmq-cli-rs.git
cd zmq-cli-rs
cargo run --release -- -vv start --routine rrbroker -1 tcp://0.0.0.0:6000 -2 tcp://0.0.0.0:6001 --socket-type proxy
```


#### CLI Help
View the help documents like top command help shows subcommands
```
jwtrueb@dhcp-10-101-176-87 vr_actuators_cli % cargo run -- --help
   Compiling vr_actuators_cli v0.1.0 (/Users/jwtrueb/Desktop/workspace/vr-actuators/vr_actuators_cli)
    Finished dev [unoptimized + debuginfo] target(s) in 4.09s
     Running `target/debug/vr_actuators_cli --help`
VR Actuators v0.1
Jacob Trueb <jtrueb@northwestern.edu
Manipulate VR Actuator Blocks

USAGE:
    vr_actuators_cli [FLAGS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -v               Sets the level of verbosity
    -V, --version    Prints version information

SUBCOMMANDS:
    command    Executes a command for VR Actuators
    help       Prints this message or the help of the given subcommand(s)
    start      Starts the service that manages the connection to the VR Actuators
```

subcommand help
```
jwtrueb@dhcp-10-101-176-87 vr_actuators_cli % cargo run -- start --help
    Finished dev [unoptimized + debuginfo] target(s) in 0.04s
     Running `target/debug/vr_actuators_cli start --help`
vr_actuators_cli-start
Starts the service that manages the connection to the VR Actuators

USAGE:
    vr_actuators_cli start [OPTIONS]

FLAGS:
        --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -h, --hostname <HOSTNAME>    Sets hostname to bind for communication [default: *]
    -p, --port <PORT>            Sets port to bind for communication [default: 5555]
        --protocol <PROTOCOL>    Sets ZMQ protocol for the server [default: tcp]
```


#### Example Server Output
```
jwtrueb@dhcp-10-101-176-87 vr_actuators_cli % cargo run --release -- -vv start --protocol tcp --hostname ubuntu20 --port 6001
    Finished release [optimized] target(s) in 0.04s
     Running `target/release/vr_actuators_cli -vv start --protocol tcp --hostname ubuntu20 --port 6001`
2020-09-02 15:12:17,703 DEBUG [vr_actuators_cli] Found level_filter: DEBUG
2020-09-02 15:12:17,704 INFO  [vr_actuators_cli] Starting up ...
2020-09-02 15:12:17,704 INFO  [vr_actuators_cli::network] Connected to tcp://ubuntu20:6001
2020-09-02 15:12:17,712 DEBUG [vr_actuators_cli::network::vrp] Found Obid/Feig USB Device || Bus 020 Device 009 ID 2737 : 2
2020-09-02 15:12:17,782 DEBUG [vr_actuators_cli::network::vrp] Claiming interface: 0
2020-09-02 15:12:17,784 DEBUG [vr_actuators_cli::network::vrp] Claiming interface: 1
2020-09-02 15:12:17,784 INFO  [vr_actuators_cli::network] Beginning serve() loop ...
2020-09-02 15:12:22,733 INFO  [vr_actuators_cli::network] Received SetRadioFreqPower command for power_level 0.
2020-09-02 15:12:22,740 DEBUG [vr_actuators_cli::network::vrp] Sent Serial Command with 44 bytes: 02002cff8b020101011e0003000884800000000000000000008100000000000000000000000000000000a7e6
2020-09-02 15:12:22,760 DEBUG [vr_actuators_cli::network::vrp] Received Response to Serial Command with 8 bytes: 020008008b009a8d
2020-09-02 15:12:22,760 INFO  [vr_actuators_cli::network::vrp] Received response: ReaderToHost {
    stx: PhantomData,
    alength: 8,
    com_adr: 0,
    control_byte: 139,
    status: 0,
    data: [],
    crc16: 36250,
}
2020-09-02 15:12:22,804 INFO  [vr_actuators_cli::network] Received SystemReset.
2020-09-02 15:12:22,810 DEBUG [vr_actuators_cli::network::vrp] Sent Serial Command with 8 bytes: 020008ff64003821
2020-09-02 15:12:22,815 DEBUG [vr_actuators_cli::network::vrp] Received Response to Serial Command with 8 bytes: 020008006400cbe7
```

#### Example Client Output
```
jwtrueb@dhcp-10-101-176-87 vr_actuators_cli % cargo run --release -- -vv command --protocol tcp --hostname ubuntu20 --port 6000 commands/set-power-0.txt
    Finished release [optimized] target(s) in 0.08s
     Running `target/release/vr_actuators_cli -vv command --protocol tcp --hostname ubuntu20 --port 6000 commands/set-power-0.txt`
2020-09-02 15:11:17,690 DEBUG [vr_actuators_cli] Found level_filter: DEBUG
2020-09-02 15:11:17,691 INFO  [vr_actuators_cli] Running command: command
2020-09-02 15:11:17,691 INFO  [vr_actuators_cli::network] Connected to tcp://ubuntu20:6000
2020-09-02 15:11:20,192 DEBUG [zmq] socket dropped
2020-09-02 15:11:20,192 DEBUG [zmq] context dropped
jwtrueb@dhcp-10-101-176-87 vr_actuators_cli % echo $?
0
```

#### Logging

There are 4 global logging levels supported by the current logging setup:
- Error is always on
- `-v`: Info
- `-vv`: Debug
- `-vvv`: Trace