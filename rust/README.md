# Rust Melody

## Dev Setup

1. Install Rust from [rustup.rs](https://rustup.rs/).
2. Install [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) for IDE support.
3. `make rust-test` && `make rust-lint` to ensure everything is working.

## Examples

The `examples/` directory contains several example programs demonstrating how to use the Melody parsing library in Rust. You can run them using:

```bash
cargo run --example basic_usage
```

## Usage

### Basic Filter

```rust
use melody_parsing::{FilterImpl, FilterOptions, Filter};

// Create a filter with options
let options = FilterOptions::new()
    .with_left_trimmed()
    .with_right_trimmed();

let mut filter = FilterImpl::new(Some(tokenizer));
options.apply_to_filter(&mut filter);

// Write tokens
let outputs = filter.write(token_id, Some(log_prob))?;

// Process outputs
for output in outputs {
    println!("Text: {}", output.text);
    for citation in output.citations {
        println!("Citation: {}", citation.text);
    }
}

// Flush remaining tokens
let final_outputs = filter.flush_partials();
```

### Stream Filter

```rust
use melody_parsing::{StreamFilter, FilterImpl, FilterOptions};

let options = FilterOptions::new()
    .handle_multi_hop_cmd3()
    .stream_tool_actions();

let filter = FilterImpl::new(Some(tokenizer));
options.apply_to_filter(&mut filter);

let stream = StreamFilter::new(filter);

// Write decoded text
stream.write_decoded("hello world", logprobs);

// Read from output channel
let rx = stream.read();
for output in rx {
    println!("{}", output.text);
}

stream.close();
```
