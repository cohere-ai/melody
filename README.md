# melody

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

### Parsing

```rust
use cohere_melody::parsing::Filter;
use cohere_melody::*;

// Create a filter with options
let options = parsing::FilterOptions::new().cmd3();

let mut filter = parsing::new_filter(options);

// Simulate text
let citation_text = "Hello <co: 1>world</co: 1>!";

// Write text — returns an aggregated result
let result = filter.write_decoded(citation_text);

if let Some(text) = &result.content {
    println!("  Content: {text}");
}
for citation in &result.citations {
    println!(
        "    Citation: {} (indices {}-{})",
        citation.text, citation.start_index, citation.end_index
    );
}

// Flush remaining tokens
let final_result = filter.flush_partials();
```

### Templating

```rust
use cohere_melody::parsing::Filter;
use cohere_melody::*;

// Assemble inputs into the template (e.g. conversation history)
let options = templating::RenderCmd4Options {
    messages: vec![
        templating::types::Message {
            role: templating::types::Role::System,
            content: vec![templating::types::Content {
                content_type: templating::types::ContentType::Text,
                text: Some("You are a helpful assistant.".to_string()),
                thinking: None,
                image: None,
                document: None,
            }],
            tool_calls: vec![],
            tool_call_id: None,
            citations: vec![],
        },
        templating::types::Message {
            role: templating::types::Role::User,
            content: vec![templating::types::Content {
                content_type: templating::types::ContentType::Text,
                text: Some("Hello Command!.".to_string()),
                thinking: None,
                image: None,
                document: None,
            }],
            tool_calls: vec![],
            tool_call_id: None,
            citations: vec![],
        },
    ],
    // Prefer capability ids: cmd4-reasoning, cmd4-classic@1, cmd5-no-escape.
    template_id: Some("cmd4-reasoning".to_string()),
    use_jinja: true,
    ..Default::default()
};

// Render prompt
let prompt = templating::render_cmd4(&options).unwrap();
```

#### Template IDs

Built-in templates use `{name}@{revision}` (same as the archive path without `.jinja`):

| Form | Example | Meaning |
|------|---------|---------|
| Exact pin | `cmd4-reasoning@1` | That revision only |
| Latest | `cmd4-reasoning` | Highest revision of that name |

For CURL, prefer archive `latest.jinja` symlinks (or pin `@N`); see
[`template_generation/ARCHIVE.md`](template_generation/ARCHIVE.md).

Build config: [`template_generation/template_registry.yaml`](template_generation/template_registry.yaml)
(`make generate-dev` → archive + `gen/embedded_templates.rs`). Melody embeds
templates from `gen/templates/archive/` (see
[`template_generation/ARCHIVE.md`](template_generation/ARCHIVE.md)). Changing
content for an existing `{name}@{revision}` fails generation until you bump
`revision` (enforced by `gen/template_revision_locks.json`).

**Default change:** empty `template_jinja` for cmd3/cmd4 now falls back to
`cmd3-reasoning` / `cmd4-reasoning`, not the former classic/legacy templates.
Pin `cmd3-legacy` or `cmd4-classic` when you need the previous default.

## Releasing

Bump the version in `Cargo.toml` and push to `main`. The CI will automatically create a GitHub release, build wheels for all platforms, and publish to PyPI.

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

## Debugging

You may run into issues calling the Rust static library from other languages (e.g. Golang via CGO). One effective way to debug these issues is to:

1. Build the library in debug mode. You can do this by adding these lines to the `Cargo.toml` file:

```toml
[profile.release]
debug = true
```

and then building the library normally: `make rust-build-with-tokenizers`.

2. Create a binary to debug. In Golang, I create a binary from a test:

```bash
go test -count=1 ./gobindings/... -c
```

3. Use `gdb` (`brew install gdb` on macOS).

```bash
gdb ./gobindings.test
break melody_render_cmd3
run
```

4. When it hits the breakpoint, you can step through the Rust code to see where things might be going wrong.
