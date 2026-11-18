use std::hint::black_box;

use cohere_melody::parsing::{Filter, FilterOptions, new_filter};
use divan::counter::BytesCount;

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

fn stream_bytes(chunks: &[&str], repeats: usize) -> BytesCount {
    BytesCount::new(chunks.iter().map(|chunk| chunk.len()).sum::<usize>() * repeats)
}

fn bench_filter_construction(bencher: divan::Bencher, options: FilterOptions) {
    bencher.bench_local(|| black_box(new_filter(black_box(options.clone()))));
}

#[divan::bench]
fn new_filter_default(bencher: divan::Bencher) {
    bench_filter_construction(bencher, FilterOptions::new());
}

#[divan::bench]
fn new_filter_cmd3(bencher: divan::Bencher) {
    bench_filter_construction(bencher, FilterOptions::new().cmd3());
}

#[divan::bench]
fn new_filter_cmd4(bencher: divan::Bencher) {
    bench_filter_construction(bencher, FilterOptions::new().cmd4());
}

#[divan::bench]
fn new_filter_cmd5(bencher: divan::Bencher) {
    bench_filter_construction(bencher, FilterOptions::new().cmd5());
}

#[divan::bench(args = [1_024, 16_384, 65_536])]
fn write_decoded_plain_ascii(bencher: divan::Bencher, repeats: usize) {
    bencher
        .counter(stream_bytes(PLAIN_ASCII_CHUNKS, repeats))
        .bench_local(|| {
            run_repeated_stream(FilterOptions::new(), PLAIN_ASCII_CHUNKS, repeats);
        });
}

#[divan::bench(args = [1_024, 16_384, 65_536])]
fn write_decoded_plain_unicode(bencher: divan::Bencher, repeats: usize) {
    bencher
        .counter(stream_bytes(PLAIN_UNICODE_CHUNKS, repeats))
        .bench_local(|| {
            run_repeated_stream(FilterOptions::new(), PLAIN_UNICODE_CHUNKS, repeats);
        });
}

#[divan::bench(args = [1, 8, 256, 4_096, 8_192])]
fn write_decoded_cmd4_reasoning_stream(bencher: divan::Bencher, repeats: usize) {
    bencher
        .counter(stream_bytes(CMD4_REASONING_CHUNKS, repeats))
        .bench_local(|| {
            run_repeated_stream(
                FilterOptions::new().cmd4().no_tools(),
                CMD4_REASONING_CHUNKS,
                repeats,
            );
        });
}

#[divan::bench(args = [1, 8, 128, 1_024, 7_280])]
fn write_decoded_cmd4_tool_stream(bencher: divan::Bencher, repeats: usize) {
    bencher
        .counter(stream_bytes(CMD4_TOOL_CHUNKS, repeats))
        .bench_local(|| {
            run_repeated_stream(FilterOptions::new().cmd4(), CMD4_TOOL_CHUNKS, repeats);
        });
}
