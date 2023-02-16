FROM ubuntu:20.04

MAINTAINER Alexey Pashinov <pashinov93@gmail.com>

WORKDIR /root/contracts

# Common packages
ARG DEBIAN_FRONTEND=noninteractive

RUN apt update && \
    apt install --no-install-recommends -y \
    build-essential libssl-dev libudev-dev gcc clang \
    cmake ca-certificates bzip2 wget curl pkg-config

# Install Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain 1.67.0 -y

ENV PATH="/root/.cargo/bin:${PATH}"

# Install WASM
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Intall Solana tools
ARG SOLANA_VERSION=1.13.6

RUN wget -O /opt/solana-${SOLANA_VERSION}.tar.gz https://github.com/solana-labs/solana/archive/refs/tags/v${SOLANA_VERSION}.tar.gz

RUN cd /opt; tar -xvf solana-${SOLANA_VERSION}.tar.gz
RUN cd /opt/solana-${SOLANA_VERSION}; ./scripts/cargo-install-all.sh .

ENV PATH=/opt/solana-${SOLANA_VERSION}/bin:$PATH

# There can be only one CMD instruction
CMD ["/bin/bash"]
