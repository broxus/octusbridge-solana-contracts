# Solana Bridge

#### Build
```bash
cargo build-bpf --manifest-path=./token-proxy/Cargo.toml --bpf-out-dir=dist/program
```

#### Deploy
```bash
solana program deploy ./dist/program/token_proxy.so
```

#### Run tests
```bash
cargo test-bpf --manifest-path=./token-proxy/Cargo.toml
```

#### Build WASM bindings
```bash
wasm-pack build --target web --out-name index token-proxy -- --features wasm
```

#### Build Rust bindings
```bash
cargo build --release --manifest-path=./token-proxy/Cargo.toml --features=bindings
```
