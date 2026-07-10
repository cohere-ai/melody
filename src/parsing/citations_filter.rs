//! Citation parsing functionality
//!
//! This module handles parsing of inline citations from grounded model outputs.
//! Citations are used to attribute generated text to specific source documents or
//! tool results.
//!

use crate::parsing::filter::{FilterImpl, PartialMatchResult, find_partial};
use crate::parsing::types::{FilterCitation, FilterMode, FilterOutput, Source};

// Citation marker constants
const START_FIRST_CIT: &str = "<co: ";
const START_LAST_CIT: &str = "</co: ";
const END_OF_CIT: &str = ">";
const START_OF_CIT: char = '<';
const START_FIRST_CIT_CMD3: &str = "<co";

impl FilterImpl {
    fn emit_plain_text_citation_output(&mut self, s: &str) -> (Option<FilterOutput>, usize) {
        self.cur_text_index += s.chars().count();
        self.cur_text_byte_index += s.len();
        (
            Some(FilterOutput {
                text: s.to_string(),
                ..Default::default()
            }),
            s.len(),
        )
    }

    /// Process text response, extracting citations.
    ///
    /// This method is called when in `GroundedAnswer` or `ToolReason` mode. It parses
    /// the text stream looking for citation markers and extracts both the text
    /// and source attribution.
    ///
    /// # Arguments
    ///
    /// * `bstr` - Byte string to process
    /// * `after_last_token` - Whether this is the final flush (affects buffering)
    /// * `mode` - Current filter mode (`GroundedAnswer` or `ToolReason`)
    ///
    /// # Returns
    ///
    /// A tuple of (outputs, `bytes_consumed`) where `bytes_consumed` indicates how
    /// many bytes from bstr were processed and can be removed from the buffer.
    pub(crate) fn process_grounded_text(
        &mut self,
        bstr: &[u8],
        after_last_token: bool,
        mode: FilterMode,
    ) -> (Vec<FilterOutput>, usize) {
        if !Self::utf8_valid_or_limit(bstr) {
            return (Vec::new(), 0);
        }

        let lossy = String::from_utf8_lossy(bstr);
        let (send, rem_right) = self.trim_space(&lossy);
        let remove = bstr.len() - send.len() - rem_right;

        let (mut res_out, remove_cit) = self.parse_citations(send, mode);

        if res_out.is_none()
            || (res_out.as_ref().unwrap().text.is_empty()
                && res_out.as_ref().unwrap().citations.is_empty())
        {
            if send.is_empty() || !after_last_token {
                return (Vec::new(), remove + remove_cit);
            }
            res_out = Some(FilterOutput {
                text: send.to_string(),
                ..Default::default()
            });
        }

        let mut res_out = res_out.unwrap();
        res_out.is_post_answer = self.stream_non_grounded_answer && mode != FilterMode::ToolReason;
        res_out.is_reasoning = mode == FilterMode::ToolReason;

        let mut out = Vec::new();
        if self.stream_tool_actions || !res_out.is_reasoning {
            out.push(res_out);
        }

        (out, remove + remove_cit)
    }

    pub(crate) fn parse_citations(
        &mut self,
        s: &str,
        mode: FilterMode,
    ) -> (Option<FilterOutput>, usize) {
        if !s.contains(START_OF_CIT) {
            return self.emit_plain_text_citation_output(s);
        }

        let start_first_citation_str = if self.cmd3_citations {
            START_FIRST_CIT_CMD3
        } else {
            START_FIRST_CIT
        };

        let (start_first_id, end_first_id, _) =
            Self::find_an_element(s, start_first_citation_str, END_OF_CIT, self.cmd3_citations);

        // No citation was found so send the plain text and remove from buffer
        if start_first_id == usize::MAX {
            return self.emit_plain_text_citation_output(s);
        }

        // Only partial citation found so we need to wait for the complete citation.
        if end_first_id == usize::MAX {
            return (None, 0);
        }

        // Then try to find the 'last' citation element.
        let (start_last_id, end_last_id, docs_last) =
            Self::find_an_element(s, START_LAST_CIT, END_OF_CIT, self.cmd3_citations);

        // Only partial citation found so we need to wait for the complete citation.
        if start_last_id == usize::MAX || end_last_id == usize::MAX {
            if !self.stream_non_grounded_answer && end_last_id == usize::MAX {
                let (txt, remove) = self.get_partial_or_malformed_citation_text(
                    start_first_id,
                    end_first_id,
                    start_last_id,
                    s,
                );
                if !txt.is_empty() {
                    return (
                        Some(FilterOutput {
                            text: txt,
                            ..Default::default()
                        }),
                        remove,
                    );
                }
            }
            return (None, 0);
        }

        if end_first_id > start_last_id {
            log::warn!(
                "Invalid citation: text={s}, start_first_id={start_first_id}, start_last_id={start_last_id}"
            );
            return (None, 0);
        }

        // We have found a whole citation, now find the indexes for the citation
        let start_index = self.cur_text_index + s[0..start_first_id].chars().count();
        let end_of_cit = end_last_id + 1;
        let cit_txt = &s[end_first_id + 1..start_last_id];
        let mut text = format!("{}{}", &s[..start_first_id], cit_txt);
        self.cur_text_index += text.chars().count();
        self.cur_text_byte_index += text.len();

        if let Some(start_idx) = self.cur_citation_byte_index {
            if start_idx < start_last_id {
                text = s[start_idx..start_last_id].to_string();
            } else {
                text = String::new();
            }
        }
        self.cur_citation_byte_index = None;

        let mut sources = docs_last;
        self.resolve_document_ids(&mut sources);
        let mut cits = vec![FilterCitation {
            start_index,
            end_index: start_index + cit_txt.chars().count(),
            text: cit_txt.to_string(),
            sources,
            is_thinking: mode == FilterMode::ToolReason,
        }];

        // Recurse to find more partial or complete citations
        let (more_cits, more_rem) = self.parse_citations(&s[end_of_cit..], mode);
        if let Some(more_cits) = more_cits {
            cits.extend(more_cits.citations);
            text.push_str(&more_cits.text);
        }

        (
            Some(FilterOutput {
                text,
                citations: cits,
                ..Default::default()
            }),
            end_of_cit + more_rem,
        )
    }

    fn get_partial_citation_text(
        &mut self,
        start_first_id: usize,
        end_first_id: usize,
        start_last_id: usize,
        s: &str,
    ) -> (String, usize) {
        let text_before_citation = &s[..start_first_id];
        self.cur_text_index += text_before_citation.chars().count();
        self.cur_text_byte_index += text_before_citation.len();

        let start_idx = if let Some(start_idx) = self.cur_citation_byte_index {
            // If we've already processed all of this string, return early
            if start_idx >= s.len() {
                return (text_before_citation.to_string(), text_before_citation.len());
            }
            start_idx
        } else {
            end_first_id + 1
        };

        let byte_offset = s.len().saturating_sub(text_before_citation.len());
        self.cur_citation_byte_index = Some(byte_offset);

        let end_idx = if start_last_id != usize::MAX && start_last_id > 0 {
            start_last_id
        } else {
            s.len()
        };

        if start_idx >= end_idx {
            return (text_before_citation.to_string(), text_before_citation.len());
        }

        (
            format!("{}{}", text_before_citation, &s[start_idx..end_idx]),
            text_before_citation.len(),
        )
    }

    fn get_partial_or_malformed_citation_text(
        &mut self,
        start_first_id: usize,
        end_first_id: usize,
        start_last_id: usize,
        s: &str,
    ) -> (String, usize) {
        if !self.cmd3_citations || START_FIRST_CIT_CMD3.len() + start_first_id == end_first_id {
            return self.get_partial_citation_text(start_first_id, end_first_id, start_last_id, s);
        }

        let txt = if start_last_id != usize::MAX && start_last_id > 0 {
            &s[..start_last_id]
        } else {
            s
        };

        self.cur_text_index += txt.chars().count();
        self.cur_text_byte_index += txt.len();

        (txt.to_string(), txt.len())
    }

    fn find_an_element(
        s: &str,
        start: &str,
        end: &str,
        cmd3_citations: bool,
    ) -> (usize, usize, Vec<Source>) {
        let start_id = match find_partial(s, [start.to_string()].iter()) {
            PartialMatchResult::NoMatch => return (usize::MAX, usize::MAX, Vec::new()),
            PartialMatchResult::Partial { idx } => return (idx, usize::MAX, Vec::new()),
            PartialMatchResult::Full { idx, .. } => idx,
        };

        let Some(end_id) = s[start_id + 1..].find(end) else {
            return (start_id, usize::MAX, Vec::new());
        };

        let substring = &s[start_id + start.len()..start_id + 1 + end_id];

        let doc_indices = if cmd3_citations {
            Self::convert_string_to_doc_indices(substring)
        } else {
            let int_indices = convert_string_to_int_list(substring);
            if int_indices.is_empty() {
                Vec::new()
            } else {
                vec![Source {
                    tool_call_index: 0,
                    tool_result_indices: int_indices,
                    document_ids: Vec::new(),
                }]
            }
        };

        (start_id, start_id + 1 + end_id, doc_indices)
    }

    /// Fill in the original document identifiers on each `Source`, using
    /// the lookup table the parser was configured with via
    /// [`crate::parsing::FilterOptions::with_message_history`].
    ///
    /// No-op when the parser wasn't configured with a lookup table. When
    /// a citation refers to an out-of-bounds `tool_call_index` the source
    /// is left unresolved (`document_ids` stays empty); for out-of-bounds
    /// `tool_result_indices` the corresponding entry becomes an empty
    /// string, preserving positional alignment with `tool_result_indices`.
    pub(crate) fn resolve_document_ids(&self, sources: &mut [Source]) {
        if self.document_ids.is_empty() {
            return;
        }
        for source in sources {
            let Some(row) = self.document_ids.get(source.tool_call_index) else {
                continue;
            };
            source.document_ids = source
                .tool_result_indices
                .iter()
                .map(|&idx| row.get(idx).cloned().unwrap_or_default())
                .collect();
        }
    }

    fn convert_string_to_doc_indices(s: &str) -> Vec<Source> {
        let string_splits: Vec<&str> = s.trim().split(']').collect();
        let mut doc_indices = Vec::new();

        for cit in &string_splits[..string_splits.len().saturating_sub(1)] {
            let cit_splits: Vec<&str> = cit.trim_start_matches(',').split(':').collect();
            if cit_splits.len() != 2 {
                log::warn!(
                    "Invalid citation, not 2 elements after split on ':': len={}",
                    cit_splits.len()
                );
                continue;
            }

            let tool_idx_str = cit_splits[0];
            let result_indices_str = cit_splits[1];

            let Ok(tool_index) = tool_idx_str.trim().parse::<usize>() else {
                log::warn!("Invalid citation tool index");
                continue;
            };

            let mut result_indices = Vec::new();
            let result_idx_splits: Vec<&str> = result_indices_str
                .trim_start_matches('[')
                .split(',')
                .collect();

            for result_split in result_idx_splits {
                match result_split.trim().parse::<usize>() {
                    Ok(idx) => result_indices.push(idx),
                    _ => {
                        log::warn!("Invalid citation result index");
                    }
                }
            }

            doc_indices.push(Source {
                tool_call_index: tool_index,
                tool_result_indices: result_indices,
                document_ids: Vec::new(),
            });
        }

        doc_indices
    }
}

fn convert_string_to_int_list(s: &str) -> Vec<usize> {
    let string_indexes: Vec<&str> = s.split(',').collect();
    let mut int_arr = Vec::new();

    for a in string_indexes {
        if let Ok(j) = a.parse::<usize>() {
            int_arr.push(j);
        }
    }

    int_arr
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::filter::FilterImpl;

    #[test]
    fn test_handle_citations_standard_case() {
        let mut filter = FilterImpl::new();
        filter.stream_non_grounded_answer = true;
        filter.cur_citation_byte_index = None;

        let input = "hello <co: 2,1>foo</co: 2,1>";
        let (output, remove) = filter.parse_citations(input, FilterMode::GroundedAnswer);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.text, "hello foo");
        assert_eq!(output.citations.len(), 1);
        assert_eq!(output.citations[0].start_index, 6);
        assert_eq!(output.citations[0].end_index, 9);
        assert_eq!(output.citations[0].text, "foo");
        assert_eq!(output.citations[0].sources.len(), 1);
        assert_eq!(output.citations[0].sources[0].tool_call_index, 0);
        assert_eq!(
            output.citations[0].sources[0].tool_result_indices,
            vec![2, 1]
        );
        assert_eq!(remove, 28);
    }

    #[test]
    fn test_handle_citations_standard_case_no_stream() {
        let mut filter = FilterImpl::new();
        filter.stream_non_grounded_answer = false;
        filter.cur_citation_byte_index = None;

        let input = "hello <co: 2,1>foo</co: 2,1>";
        let (output, remove) = filter.parse_citations(input, FilterMode::GroundedAnswer);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.text, "hello foo");
        assert_eq!(output.citations.len(), 1);
        assert_eq!(output.citations[0].start_index, 6);
        assert_eq!(output.citations[0].end_index, 9);
        assert_eq!(output.citations[0].text, "foo");
        assert_eq!(output.citations[0].sources.len(), 1);
        assert_eq!(output.citations[0].sources[0].tool_call_index, 0);
        assert_eq!(
            output.citations[0].sources[0].tool_result_indices,
            vec![2, 1]
        );
        assert_eq!(remove, 28);
    }

    #[test]
    fn test_handle_citations_multibyte() {
        let mut filter = FilterImpl::new();
        filter.stream_non_grounded_answer = false;
        filter.cur_citation_byte_index = None;

        let input = "hello🌈<co: 2,1>foo🌈</co: 2,1>.";
        let (output, remove) = filter.parse_citations(input, FilterMode::GroundedAnswer);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.text, "hello🌈foo🌈.");
        assert_eq!(output.citations.len(), 1);
        assert_eq!(output.citations[0].start_index, 6);
        assert_eq!(output.citations[0].end_index, 10);
        assert_eq!(output.citations[0].text, "foo🌈");
        assert_eq!(output.citations[0].sources.len(), 1);
        assert_eq!(output.citations[0].sources[0].tool_call_index, 0);
        assert_eq!(
            output.citations[0].sources[0].tool_result_indices,
            vec![2, 1]
        );
        assert_eq!(remove, 36);
    }

    #[test]
    fn test_handle_citations_no_document() {
        let mut filter = FilterImpl::new();
        filter.stream_non_grounded_answer = true;
        filter.cur_citation_byte_index = None;

        let input = "hello <co: >foo</co: >";
        let (output, remove) = filter.parse_citations(input, FilterMode::GroundedAnswer);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.text, "hello foo");
        assert_eq!(output.citations.len(), 1);
        assert_eq!(output.citations[0].start_index, 6);
        assert_eq!(output.citations[0].end_index, 9);
        assert_eq!(output.citations[0].text, "foo");
        assert_eq!(output.citations[0].sources.len(), 0);
        assert_eq!(remove, 22);
    }

    #[test]
    fn test_handle_citations_non_int_document() {
        let mut filter = FilterImpl::new();
        filter.stream_non_grounded_answer = true;
        filter.cur_citation_byte_index = None;

        let input = "hello <co: 2, foo>foo</co: 2, foo>";
        let (output, remove) = filter.parse_citations(input, FilterMode::GroundedAnswer);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.text, "hello foo");
        assert_eq!(output.citations.len(), 1);
        assert_eq!(output.citations[0].start_index, 6);
        assert_eq!(output.citations[0].end_index, 9);
        assert_eq!(output.citations[0].text, "foo");
        assert_eq!(output.citations[0].sources.len(), 1);
        assert_eq!(output.citations[0].sources[0].tool_call_index, 0);
        assert_eq!(output.citations[0].sources[0].tool_result_indices, vec![2]);
        assert_eq!(remove, 34);
    }

    #[test]
    fn test_handle_citations_different_documents() {
        let mut filter = FilterImpl::new();
        filter.stream_non_grounded_answer = true;
        filter.cur_citation_byte_index = None;

        let input = "hello <co: 1,2>foo</co: 3,4>";
        let (output, remove) = filter.parse_citations(input, FilterMode::GroundedAnswer);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.text, "hello foo");
        assert_eq!(output.citations.len(), 1);
        assert_eq!(output.citations[0].start_index, 6);
        assert_eq!(output.citations[0].end_index, 9);
        assert_eq!(output.citations[0].text, "foo");
        assert_eq!(output.citations[0].sources.len(), 1);
        assert_eq!(output.citations[0].sources[0].tool_call_index, 0);
        assert_eq!(
            output.citations[0].sources[0].tool_result_indices,
            vec![3, 4]
        );
        assert_eq!(remove, 28);
    }

    #[test]
    fn test_handle_citations_no_citation() {
        let mut filter = FilterImpl::new();
        filter.stream_non_grounded_answer = true;
        filter.cur_citation_byte_index = None;

        let input = "hello coo";
        let (output, remove) = filter.parse_citations(input, FilterMode::GroundedAnswer);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.text, "hello coo");
        assert_eq!(remove, 9);
    }

    #[test]
    fn test_handle_citations_no_citation_updates_ascii_indices() {
        let mut filter = FilterImpl::new();
        filter.cur_text_index = 7;
        filter.cur_text_byte_index = 7;

        let (output, remove) =
            filter.parse_citations("plain ascii text", FilterMode::GroundedAnswer);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.text, "plain ascii text");
        assert!(output.citations.is_empty());
        assert_eq!(remove, 16);
        assert_eq!(filter.cur_text_index, 23);
        assert_eq!(filter.cur_text_byte_index, 23);
    }

    #[test]
    fn test_handle_citations_no_citation_updates_multibyte_indices() {
        let mut filter = FilterImpl::new();
        filter.cur_text_index = 2;
        filter.cur_text_byte_index = 2;

        let (output, remove) = filter.parse_citations("hi🌈é", FilterMode::GroundedAnswer);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.text, "hi🌈é");
        assert!(output.citations.is_empty());
        assert_eq!(remove, "hi🌈é".len());
        assert_eq!(filter.cur_text_index, 6);
        assert_eq!(filter.cur_text_byte_index, 10);
    }

    #[test]
    fn test_handle_citations_non_citation_angle_bracket_updates_indices() {
        let mut filter = FilterImpl::new();

        let (output, remove) =
            filter.parse_citations("hello <not-a-citation>", FilterMode::GroundedAnswer);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.text, "hello <not-a-citation>");
        assert!(output.citations.is_empty());
        assert_eq!(remove, "hello <not-a-citation>".len());
        assert_eq!(
            filter.cur_text_index,
            "hello <not-a-citation>".chars().count()
        );
        assert_eq!(filter.cur_text_byte_index, "hello <not-a-citation>".len());
    }

    #[test]
    fn test_handle_citations_incomplete_first_citation() {
        let mut filter = FilterImpl::new();
        filter.stream_non_grounded_answer = true;
        filter.cur_citation_byte_index = None;

        let input = "<";
        let (output, remove) = filter.parse_citations(input, FilterMode::GroundedAnswer);

        assert!(output.is_none());
        assert_eq!(remove, 0);
    }

    #[test]
    fn test_handle_citations_multiple_citations() {
        let mut filter = FilterImpl::new();
        filter.stream_non_grounded_answer = true;
        filter.cur_citation_byte_index = None;

        let input = "hello <co: 2,1>foo</co: 2,1> hi <co: 0>barber</co: 0>";
        let (output, remove) = filter.parse_citations(input, FilterMode::GroundedAnswer);

        assert!(output.is_some());
        let output = output.unwrap();
        assert_eq!(output.text, "hello foo hi barber");
        assert_eq!(output.citations.len(), 2);

        assert_eq!(output.citations[0].start_index, 6);
        assert_eq!(output.citations[0].end_index, 9);
        assert_eq!(output.citations[0].text, "foo");
        assert_eq!(output.citations[0].sources.len(), 1);
        assert_eq!(
            output.citations[0].sources[0].tool_result_indices,
            vec![2, 1]
        );

        assert_eq!(output.citations[1].start_index, 13);
        assert_eq!(output.citations[1].end_index, 19);
        assert_eq!(output.citations[1].text, "barber");
        assert_eq!(output.citations[1].sources.len(), 1);
        assert_eq!(output.citations[1].sources[0].tool_result_indices, vec![0]);

        assert_eq!(remove, 53);
    }

    #[test]
    fn test_find_an_element_standard_case() {
        let input = "hello <co: 2,1> foo </co: 2,1>";
        let (start_index, end_index, docs) =
            FilterImpl::find_an_element(input, "<co: ", ">", false);

        assert_eq!(start_index, 6);
        assert_eq!(end_index, 14);
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].tool_call_index, 0);
        assert_eq!(docs[0].tool_result_indices, vec![2, 1]);
    }

    #[test]
    fn test_find_an_element_no_citation() {
        let input = "hello";
        let (start_index, end_index, docs) =
            FilterImpl::find_an_element(input, "<co: ", ">", false);

        assert_eq!(start_index, usize::MAX);
        assert_eq!(end_index, usize::MAX);
        assert_eq!(docs.len(), 0);
    }

    #[test]
    fn test_find_an_element_cmd3_two_tools() {
        let input = "<co> hello </co: 0:[1,2],1:[0]>";
        let (start_index, end_index, docs) =
            FilterImpl::find_an_element(input, "</co: ", ">", true);

        assert_eq!(start_index, 11);
        assert_eq!(end_index, 30);
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].tool_call_index, 0);
        assert_eq!(docs[0].tool_result_indices, vec![1, 2]);
        assert_eq!(docs[1].tool_call_index, 1);
        assert_eq!(docs[1].tool_result_indices, vec![0]);
    }

    #[test]
    fn test_convert_string_to_int_list() {
        assert_eq!(convert_string_to_int_list("0"), vec![0]);
        assert_eq!(convert_string_to_int_list("0,"), vec![0]);
        assert_eq!(convert_string_to_int_list("0,1"), vec![0, 1]);
        assert_eq!(convert_string_to_int_list("1,0"), vec![1, 0]);
        assert_eq!(convert_string_to_int_list(""), Vec::<usize>::new());
        assert_eq!(convert_string_to_int_list("foo"), Vec::<usize>::new());
        assert_eq!(convert_string_to_int_list("foo,0"), vec![0]);
        assert_eq!(convert_string_to_int_list("999"), vec![999]);
        assert_eq!(convert_string_to_int_list("-1"), Vec::<usize>::new());
    }

    mod test_resolve_document_ids {
        use crate::parsing::options::{FilterOptions, new_filter};
        use serde_json::Value;

        fn cmd3_input(citation: &str) -> String {
            format!("<|START_RESPONSE|>foo {citation}<|END_RESPONSE|>")
        }

        fn deserialize_messages(json: &str) -> Vec<crate::templating::types::Message> {
            let value: Value = serde_json::from_str(json).unwrap();
            serde_path_to_error::deserialize(&value).unwrap()
        }

        fn deserialize_documents(json: &str) -> Vec<crate::templating::types::Document> {
            let value: Value = serde_json::from_str(json).unwrap();
            serde_path_to_error::deserialize(&value).unwrap()
        }

        #[test]
        fn without_message_history_leaves_document_ids_empty() {
            let opts = FilterOptions::new().cmd3();
            let mut filter = new_filter(opts);
            let result = filter.process_full_text(&cmd3_input("<co>bar</co: 0:[0,1]>"));
            assert_eq!(result.citations.len(), 1);
            let src = &result.citations[0].sources[0];
            assert_eq!(src.tool_call_index, 0);
            assert_eq!(src.tool_result_indices, vec![0, 1]);
            assert!(
                src.document_ids.is_empty(),
                "document_ids should stay empty when the parser has no lookup"
            );
        }

        #[test]
        fn top_level_documents_resolve_ids() {
            let docs = deserialize_documents(r#"[{"id": "doc-a"}, {"id": "doc-b"}]"#);
            let opts = FilterOptions::new().cmd3().with_message_history(&[], &docs);
            let mut filter = new_filter(opts);
            let result = filter.process_full_text(&cmd3_input("<co>bar</co: 0:[0,1]>"));
            assert_eq!(result.citations.len(), 1);
            let src = &result.citations[0].sources[0];
            assert_eq!(src.document_ids, vec!["doc-a", "doc-b"]);
        }

        #[test]
        fn tool_result_documents_resolve_ids() {
            let msgs = deserialize_messages(
                r#"[
                    {"role": "chatbot", "content": [], "tool_calls": [
                        {"id": "call_1", "name": "search", "parameters": "{}"}
                    ]},
                    {"role": "tool", "tool_call_id": "call_1", "content": [
                        {"type": "document", "document": {"id": "res-x"}},
                        {"type": "document", "document": {"id": "res-y"}},
                        {"type": "document", "document": {"id": "res-z"}}
                    ]}
                ]"#,
            );
            let opts = FilterOptions::new().cmd3().with_message_history(&msgs, &[]);
            let mut filter = new_filter(opts);
            // tool_call_index 0 because no top-level docs; citing results 0 and 2.
            let result = filter.process_full_text(&cmd3_input("<co>bar</co: 0:[0,2]>"));
            let src = &result.citations[0].sources[0];
            assert_eq!(src.document_ids, vec!["res-x", "res-z"]);
        }

        #[test]
        fn top_level_docs_and_tool_calls_interleave_correctly() {
            let msgs = deserialize_messages(
                r#"[
                    {"role": "chatbot", "content": [], "tool_calls": [
                        {"id": "call_1", "name": "search", "parameters": "{}"}
                    ]},
                    {"role": "tool", "tool_call_id": "call_1", "content": [
                        {"type": "document", "document": {"id": "res-1"}}
                    ]}
                ]"#,
            );
            let docs = deserialize_documents(r#"[{"id": "doc-a"}]"#);
            let opts = FilterOptions::new()
                .cmd3()
                .with_message_history(&msgs, &docs);
            let mut filter = new_filter(opts);
            // tool_call_index 0 → the reserved top-level docs bucket;
            // tool_call_index 1 → the tool call.
            let result = filter.process_full_text(&cmd3_input("<co>bar</co: 0:[0],1:[0]>"));
            let cit = &result.citations[0];
            assert_eq!(cit.sources.len(), 2);
            assert_eq!(cit.sources[0].document_ids, vec!["doc-a"]);
            assert_eq!(cit.sources[1].document_ids, vec!["res-1"]);
        }

        #[test]
        fn out_of_bounds_tool_call_index_leaves_source_unresolved() {
            let docs = deserialize_documents(r#"[{"id": "doc-a"}]"#);
            let opts = FilterOptions::new().cmd3().with_message_history(&[], &docs);
            let mut filter = new_filter(opts);
            // tool_call_index 5 doesn't exist.
            let result = filter.process_full_text(&cmd3_input("<co>bar</co: 5:[0]>"));
            let src = &result.citations[0].sources[0];
            assert!(src.document_ids.is_empty());
        }

        #[test]
        fn out_of_bounds_tool_result_index_becomes_empty_string() {
            let docs = deserialize_documents(r#"[{"id": "doc-a"}]"#);
            let opts = FilterOptions::new().cmd3().with_message_history(&[], &docs);
            let mut filter = new_filter(opts);
            // tool_result_index 3 doesn't exist for docs bucket 0 (only doc-a).
            let result = filter.process_full_text(&cmd3_input("<co>bar</co: 0:[0,3]>"));
            let src = &result.citations[0].sources[0];
            // Positional alignment preserved.
            assert_eq!(src.document_ids, vec!["doc-a".to_string(), String::new()]);
        }

        #[test]
        fn legacy_format_resolves_via_bucket_zero() {
            let docs = deserialize_documents(r#"[{"id": "doc-a"}, {"id": "doc-b"}]"#);
            let opts = FilterOptions::new()
                .handle_rag()
                .with_message_history(&[], &docs);
            let mut filter = new_filter(opts);
            let result = filter.process_full_text("Grounded answer: <co: 0,1>hi</co: 0,1>");
            let src = &result.citations[0].sources[0];
            assert_eq!(src.tool_call_index, 0);
            assert_eq!(src.document_ids, vec!["doc-a", "doc-b"]);
        }

        #[test]
        fn invalid_message_history_is_accepted_and_resolves_best_effort() {
            // Template-shape errors (duplicate tool_call ids here) are the
            // renderer's concern; the parser must never surface a template
            // validation back to a caller. Duplicates dedupe into a single
            // bucket, and citations continue to resolve against it.
            let msgs = deserialize_messages(
                r#"[
                    {"role": "chatbot", "content": [], "tool_calls": [
                        {"id": "call_1", "name": "search", "parameters": "{}"}
                    ]},
                    {"role": "chatbot", "content": [], "tool_calls": [
                        {"id": "call_1", "name": "search", "parameters": "{}"}
                    ]},
                    {"role": "tool", "tool_call_id": "call_1", "content": [
                        {"type": "document", "document": {"id": "res-1"}}
                    ]}
                ]"#,
            );
            let opts = FilterOptions::new().cmd3().with_message_history(&msgs, &[]);
            let mut filter = new_filter(opts);
            let result = filter.process_full_text(&cmd3_input("<co>bar</co: 0:[0]>"));
            let src = &result.citations[0].sources[0];
            assert_eq!(src.document_ids, vec!["res-1"]);
        }
    }
}
