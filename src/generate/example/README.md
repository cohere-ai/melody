# Example usage of the generate_tool

This directory contains a simple example to demonstrate how the tool works.

## Files

- `example_config.yaml`: Example input configuration
- `example_template.tmpl`: Example template file
- `example_variable.tmpl`: Example variable file

## Running the Example

```bash
# From the rust_generate_tool directory
cargo run -- --config-dir example --in-file example_config.yaml --out-file example_output.yaml

# Or with the compiled binary
./target/release/generate_tool --config-dir example --in-file example_config.yaml --out-file example_output.yaml
```

This will generate `example/example_output.yaml` with the rendered templates.

