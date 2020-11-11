FROM ubuntu:20.04

ARG DEBIAN_FRONTEND=noninteractive

RUN date

RUN apt update && \
    apt install -y apt-utils gcc build-essential glances htop vim tree curl \
        pkg-config \
        libzmq3-dev \
        libusb-1.0-0-dev && \
    apt clean

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /home/rustup.sh && \
    chmod +x /home/rustup.sh && \
    /home/rustup.sh -y && \
    . $HOME/.cargo/env && \
    echo ". $HOME/.cargo/env" >> $HOME/.shrc

RUN mkdir -p /home/app && \
    echo "RUST_LOG=trace" >> /home/app/.env

COPY src /home/app/src/
COPY tests /home/app/tests/
COPY commands /home/app/commands/
COPY Cargo.toml Cargo.lock /home/app/

ARG CARGO_FLAGS
RUN cd /home/app && . $HOME/.shrc && \
    cargo build ${CARGO_FLAGS} && \
    cargo test ${CARGO_FLAGS}

RUN cd /home/app && . $HOME/.shrc && \
    cargo check && \
    cargo check --features "usb" && \
    cargo check --features "ethernet" && \
    cargo check --features "haptic_v0 usb" && \
    cargo check --features "haptic_v0 ethernet" && \
    cargo check --features "haptic_v0 ethernet" --release && \
    cargo check --release

RUN date
