#!/usr/bin/env bash

function print_help() {
  echo 'Usage: build.sh [OPTIONS]'
  echo ''
  echo 'Options:'
  echo '  -h,--help         Print this help message and exit'
  echo '  -p,--programs     Build solana programs'
  echo '  -w,--wasm         Build WASM bindings'
  echo '  -b,--bindings     Build Rust bindings'
  echo '  -t,--tests        Run tests'
}

while [[ $# -gt 0 ]]; do
  key="$1"
  case $key in
      -h|--help)
        print_help
        exit 0
      ;;
      -p|--programs)
        shift # past argument

        cargo-build-bpf --manifest-path=./token-proxy/Cargo.toml  --bpf-out-dir=dist/program
        cargo-build-bpf --manifest-path=./round-loader/Cargo.toml --bpf-out-dir=dist/program
      ;;
      -w|--wasm)
        shift # past argument

        wasm-pack build --target web --out-name index token-proxy  -- --features wasm
        wasm-pack build --target web --out-name index round-loader -- --features wasm
      ;;
      -b|--bindings)
        shift # past argument

        cargo build --release --manifest-path=./token-proxy/Cargo.toml  --features=bindings
        cargo build --release --manifest-path=./round-loader/Cargo.toml --features=bindings
      ;;
      -t|--tests)
        shift # past argument

        cargo-test-bpf --manifest-path=./token-proxy/Cargo.toml
        cargo-test-bpf --manifest-path=./round-loader/Cargo.toml
      ;;
      *) # unknown option
        echo 'ERROR: Unexpected'
        exit 1
      ;;
  esac
done
