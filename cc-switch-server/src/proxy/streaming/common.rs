use bytes::Bytes;
use serde_json::{json, Value};

// Keep raw bytes until an entire SSE event arrives: a UTF-8 character can span
// HTTP chunks. Decode once per event, including CRLF-framed upstream streams.
pub(super) fn take_sse_block(buffer: &mut Vec<u8>) -> Option<String> {
    let delimiter = buffer
        .windows(2)
        .position(|w| w == b"\n\n")
        .map(|i| (i, 2))
        .into_iter()
        .chain(
            buffer
                .windows(4)
                .position(|w| w == b"\r\n\r\n")
                .map(|i| (i, 4)),
        )
        .min_by_key(|(i, _)| *i)?;
    let block = String::from_utf8_lossy(&buffer[..delimiter.0]).into_owned();
    buffer.drain(..delimiter.0 + delimiter.1);
    Some(block)
}

pub(super) fn parse_sse_block(block: &str) -> Option<(Option<String>, String)> {
    let mut event = None;
    let mut data = Vec::new();
    for line in block.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            event = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("data:") {
            data.push(value.trim_start().to_string());
        }
    }
    if data.is_empty() {
        None
    } else {
        Some((event, data.join("\n")))
    }
}

pub(super) fn message_start(message_id: &str, model: &str) -> Bytes {
    sse_event(
        "message_start",
        json!({
            "type": "message_start",
            "message": {
                "id": message_id,
                "type": "message",
                "role": "assistant",
                "model": model,
                "content": [],
                "usage": { "input_tokens": 0, "output_tokens": 0 }
            }
        }),
    )
}

pub(super) fn sse_event(event: &str, data: Value) -> Bytes {
    Bytes::from(format!("event: {event}\ndata: {}\n\n", data))
}
