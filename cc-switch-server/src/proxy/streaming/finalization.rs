use super::common::sse_event;
use bytes::Bytes;
use serde_json::{json, Value};

pub(super) fn close_text_block_if_needed(
    sent_text_start: bool,
    sent_text_stop: &mut bool,
    text_block_index: Option<u32>,
) -> Option<Bytes> {
    if sent_text_start && !*sent_text_stop {
        *sent_text_stop = true;
        let index = text_block_index.unwrap_or(0);
        return Some(sse_event(
            "content_block_stop",
            json!({
                "type": "content_block_stop",
                "index": index
            }),
        ));
    }
    None
}

pub(super) fn close_thinking_block_if_needed(
    thinking_block_index: &mut Option<u32>,
) -> Option<Bytes> {
    thinking_block_index.take().map(|index| {
        sse_event(
            "content_block_stop",
            json!({
                "type": "content_block_stop",
                "index": index
            }),
        )
    })
}

pub(super) fn message_stop_event(stop_reason: &str, usage: Value) -> [Bytes; 2] {
    [
        sse_event(
            "message_delta",
            json!({
                "type": "message_delta",
                "delta": {
                    "stop_reason": stop_reason,
                    "stop_sequence": Value::Null
                },
                "usage": usage
            }),
        ),
        sse_event("message_stop", json!({ "type": "message_stop" })),
    ]
}
