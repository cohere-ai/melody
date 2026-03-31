use std::hint::black_box;

use cohere_melody::parsing::{Filter, FilterOptions, new_filter};

const PLAIN_ASCII_CHUNKS: &[&str] = &["plain text chunk "];
const PLAIN_UNICODE_CHUNKS: &[&str] = &["回答🌈", "終わりです", "ありがとう"];
const CMD4_REASONING_CHUNKS: &[&str] = &[
    "<|START_THINKING|>",
    "Réflexion étape 1. ",
    "次の手順です。 ",
    "<|END_THINKING|>",
    "<|START_TEXT|>",
    "最終回答です。 ",
    "追加の説明です。 ",
    "<|END_TEXT|>",
];
const CMD4_TOOL_CHUNKS: &[&str] = &[
    "<|START_THINKING|>",
    "Need a tool. ",
    "<|END_THINKING|>",
    "<|START_ACTION|>",
    r#"[{"tool_call_id":"call_0","tool_name":"web_search","parameters":{"query":"weather in Paris"}}]"#,
    "<|END_ACTION|>",
    "<|START_TEXT|>",
    "Temps ensoleillé. ",
    "<|END_TEXT|>",
];

fn main() {
    divan::main();
}

fn run_repeated_stream(options: FilterOptions, chunks: &[&str], repeats: usize) {
    let mut filter = new_filter(options);
    for _ in 0..repeats {
        for &chunk in chunks {
            black_box(filter.write_decoded(black_box(chunk)));
        }
    }
    black_box(filter.flush_partials());
}

#[divan::bench(args = [1_024, 16_384, 65_536])]
fn write_decoded_plain_ascii(bencher: divan::Bencher, repeats: usize) {
    bencher.bench_local(|| {
        run_repeated_stream(FilterOptions::new(), PLAIN_ASCII_CHUNKS, repeats);
    });
}

#[divan::bench(args = [1_024, 16_384, 65_536])]
fn write_decoded_plain_unicode(bencher: divan::Bencher, repeats: usize) {
    bencher.bench_local(|| {
        run_repeated_stream(FilterOptions::new(), PLAIN_UNICODE_CHUNKS, repeats);
    });
}

#[divan::bench(args = [256, 4_096, 8_192])]
fn write_decoded_cmd4_reasoning_stream(bencher: divan::Bencher, repeats: usize) {
    bencher.bench_local(|| {
        run_repeated_stream(
            FilterOptions::new().cmd4().no_tools(),
            CMD4_REASONING_CHUNKS,
            repeats,
        );
    });
}

#[divan::bench(args = [128, 1_024, 7_280])]
fn write_decoded_cmd4_tool_stream(bencher: divan::Bencher, repeats: usize) {
    bencher.bench_local(|| {
        run_repeated_stream(FilterOptions::new().cmd4(), CMD4_TOOL_CHUNKS, repeats);
    });
}
