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

        cargo-build-sbf --manifest-path=./token-proxy/Cargo.toml --sbf-out-dir=dist/program
        cargo-build-sbf --manifest-path=./round-loader/Cargo.toml --sbf-out-dir=dist/program
        cargo-build-sbf --manifest-path=./native-proxy/Cargo.toml --sbf-out-dir=dist/program
      ;;
      -b|--bindings)
        shift # past argument

        cargo build --release --manifest-path=./token-proxy/Cargo.toml  --features=bindings
        cargo build --release --manifest-path=./round-loader/Cargo.toml --features=bindings
        cargo build --release --manifest-path=./native-proxy/Cargo.toml --features=bindings
      ;;
      -w|--wasm)
        shift # past argument

        wasm-pack build --target web --out-name index wasm
        wasm-pack build --target web --out-name index round-loader -- --features wasm
      ;;
      -t|--tests)
        shift # past argument

        cargo-test-sbf --manifest-path=./token-proxy/Cargo.toml
        cargo-test-sbf --manifest-path=./round-loader/Cargo.toml
        #cargo-test-sbf --manifest-path=./native-proxy/Cargo.toml
      ;;
      *) # unknown option
        echo 'ERROR: Unexpected'
        exit 1
      ;;
  esac
done
