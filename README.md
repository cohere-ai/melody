# melody

> [!WARNING]
> This library is currently in development and the interfaces are subject to change. Be sure to pin this dependency to a specific version.

Templating rendering and generation parsing for Cohere models.

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
let options = FilterOptions::new().cmd3();

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

### Prerequisites (from [pyo3](https://pyo3.rs/v0.27.1/getting-started.html#installation))

1. Recommended [install uv](https://docs.astral.sh/uv)
2. From root directory, run:
   ```bash
   make python-bindings
   ```
3. Test the bindings:
   ```bash
   cd rust
   uv run python -c "import cohere_melody;"
   ```
