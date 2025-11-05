# Rust Melody

## Dev Setup

1. Install Rust from [rustup.rs](https://rustup.rs/).
2. Install [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) for IDE support.
3. `make rust-test` && `make rust-lint` to ensure everything is working.

## Examples

The `examples/` directory contains several example programs demonstrating how to use the Melody parsing library in Rust. You can run them using:

```bash
cargo run --example basic
```

## Usage

### Basic Filter

```rust
use cohere_melody::{FilterOptions, TokenIDsWithLogProb, new_filter};

// Create a filter with options
let options = FilterOptions::new()
    .handle_multi_hop_cmd3()
    .stream_tool_actions();

let mut filter = new_filter(options);

// Simulate text
let text = "Hello <co: 1>world</co: 1>!";
let logprobs = TokenIDsWithLogProb {
    token_ids: vec![1, 2, 3],
    logprobs: vec![0.1, 0.2, 0.3],
};

// Write text
let outputs = filter.write_decoded(text, logprobs);

// Process outputs
for output in outputs {
    println!("  Text: {}", output.text);
    for citation in output.citations {
        println!(
            "    Citation: {} (indices {}-{})",
            citation.text, citation.start_index, citation.end_index
        );
    }
}

// Flush remaining tokens
let final_outputs = filter.flush_partials();
```

## Building Python Bindings

### Prerequisites
1. [Recommended] [Install pyenv](https://github.com/pyenv/pyenv#a-getting-pyenv)
   2. Use the automatic installer under Linux/Unix; it also installs pyenv-virtualenv
2. [Recommended] Start and activate a virtualenv:
```bash
pyenv install 3.12 --keep
pyenv virtualenv 3.12 venv
pyenv activate venv
```
3. Install maturin:
```bash
pip install maturin
```

You should now be able to build the Python bindings with:
```bash
maturin develop --features python_ffi
```