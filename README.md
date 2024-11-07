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

## Run LLM (llama2.c)

The current directory (and its children) is mounted to the wasm runtime, so you can run the LLM example like this:

```bash
cd examples/llama2-c
cargo run llama2-c.wasm -- model.bin -n 256 -i 'Once upon a time'
```

# License

MIT
