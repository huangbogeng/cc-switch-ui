use bytes::Bytes;
use serde_json::{json, Value};

pub(super) fn take_sse_block(buffer: &mut String) -> Option<String> {
    let index = buffer.find("\n\n")?;
    let block = buffer[..index].to_string();
    buffer.drain(..index + 2);
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
