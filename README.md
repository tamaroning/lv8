# LV8

An experimental wasm runtime leveraging V8.

# Build

```bash
git clone git@github.com:tamaroning/lv8.git
cd lv8
V8_FORCE_DEBUG=true cargo build
```

# Run

```bash
cargo run <WASM FILE>
```

To run the hello world example:

```bash
cargo run hello.wasm
```

# License

MIT
