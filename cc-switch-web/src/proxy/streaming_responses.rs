//! OpenAI Responses SSE to Anthropic Messages SSE conversion.

use bytes::Bytes;
use futures::{Stream, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn responses_sse_to_anthropic<E>(
    stream: impl Stream<Item = Result<Bytes, E>> + Send + 'static,
) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send
where
    E: std::error::Error + Send + 'static,
{
    async_stream::stream! {
        let mut buffer = String::new();
        let mut message_id = String::new();
        let mut model = String::new();
        let mut sent_message_start = false;
        let mut next_index: u32 = 0;
        let mut text_index: Option<u32> = None;
        let mut tool_indices: HashMap<String, u32> = HashMap::new();
        let mut pending_tool_args: HashMap<String, String> = HashMap::new();

        tokio::pin!(stream);

        while let Some(chunk) = stream.next().await {
            let bytes = match chunk {
                Ok(bytes) => bytes,
                Err(err) => {
                    yield Err(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()));
                    continue;
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(block) = take_sse_block(&mut buffer) {
                let (event_name, data) = match parse_sse_block(&block) {
                    Some(parsed) => parsed,
                    None => continue,
                };
                let data: Value = match serde_json::from_str(&data) {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                match event_name.as_deref() {
                    Some("response.created") => {
                        let response = data.get("response").unwrap_or(&data);
                        if let Some(id) = response.get("id").and_then(Value::as_str) {
                            message_id = id.to_string();
                        }
                        if let Some(response_model) = response.get("model").and_then(Value::as_str) {
                            model = response_model.to_string();
                        }
                        sent_message_start = true;
                        yield Ok(sse_event("message_start", json!({
                            "type": "message_start",
                            "message": {
                                "id": message_id,
                                "type": "message",
                                "role": "assistant",
                                "model": model,
                                "content": [],
                                "usage": usage_from_responses(response.get("usage")),
                            }
                        })));
                    }
                    Some("response.content_part.added") => {
                        if !sent_message_start {
                            sent_message_start = true;
                            yield Ok(message_start(&message_id, &model));
                        }
                        let part_type = data
                            .get("part")
                            .and_then(|part| part.get("type"))
                            .and_then(Value::as_str);
                        if matches!(part_type, Some("output_text") | Some("refusal")) && text_index.is_none() {
                            let index = next_index;
                            next_index += 1;
                            text_index = Some(index);
                            yield Ok(sse_event("content_block_start", json!({
                                "type": "content_block_start",
                                "index": index,
                                "content_block": { "type": "text", "text": "" }
                            })));
                        }
                    }
                    Some("response.output_text.delta") | Some("response.refusal.delta") => {
                        if !sent_message_start {
                            sent_message_start = true;
                            yield Ok(message_start(&message_id, &model));
                        }
                        let index = match text_index {
                            Some(index) => index,
                            None => {
                                let index = next_index;
                                next_index += 1;
                                text_index = Some(index);
                                yield Ok(sse_event("content_block_start", json!({
                                    "type": "content_block_start",
                                    "index": index,
                                    "content_block": { "type": "text", "text": "" }
                                })));
                                index
                            }
                        };
                        let delta = data
                            .get("delta")
                            .and_then(Value::as_str)
                            .unwrap_or_default();
                        yield Ok(sse_event("content_block_delta", json!({
                            "type": "content_block_delta",
                            "index": index,
                            "delta": { "type": "text_delta", "text": delta }
                        })));
                    }
                    Some("response.output_item.added") => {
                        if let Some(item) = data.get("item") {
                            if item.get("type").and_then(Value::as_str) == Some("function_call") {
                                if !sent_message_start {
                                    sent_message_start = true;
                                    yield Ok(message_start(&message_id, &model));
                                }
                                let item_id = item
                                    .get("id")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .to_string();
                                let call_id = item
                                    .get("call_id")
                                    .and_then(Value::as_str)
                                    .filter(|value| !value.is_empty())
                                    .unwrap_or(&item_id)
                                    .to_string();
                                let name =
                                    item.get("name").and_then(Value::as_str).unwrap_or_default();
                                let index = next_index;
                                next_index += 1;
                                tool_indices.insert(call_id.clone(), index);
                                if !item_id.is_empty() {
                                    tool_indices.insert(item_id, index);
                                }
                                yield Ok(sse_event("content_block_start", json!({
                                    "type": "content_block_start",
                                    "index": index,
                                    "content_block": {
                                        "type": "tool_use",
                                        "id": call_id,
                                        "name": name,
                                        "input": {}
                                    }
                                })));
                            }
                        }
                    }
                    Some("response.function_call_arguments.delta") => {
                        let call_id = response_call_id(&data);
                        if let Some(call_id) = call_id {
                            let delta = data.get("delta").and_then(Value::as_str).unwrap_or_default();
                            pending_tool_args.entry(call_id).or_default().push_str(delta);
                        }
                    }
                    Some("response.output_item.done") => {
                        if let Some(item) = data.get("item") {
                            if item.get("type").and_then(Value::as_str) == Some("function_call") {
                                let item_id = item
                                    .get("id")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .to_string();
                                let call_id = item
                                    .get("call_id")
                                    .and_then(Value::as_str)
                                    .filter(|value| !value.is_empty())
                                    .unwrap_or(&item_id)
                                    .to_string();
                                let index = tool_indices
                                    .get(&call_id)
                                    .copied()
                                    .or_else(|| tool_indices.get(&item_id).copied());
                                if let Some(index) = index {
                                    let args = item
                                        .get("arguments")
                                        .and_then(Value::as_str)
                                        .map(ToString::to_string)
                                        .or_else(|| pending_tool_args.remove(&call_id))
                                        .or_else(|| pending_tool_args.remove(&item_id))
                                        .unwrap_or_default();
                                    let partial_json = serde_json::from_str::<Value>(&args).unwrap_or_else(|_| json!({}));
                                    yield Ok(sse_event("content_block_delta", json!({
                                        "type": "content_block_delta",
                                        "index": index,
                                        "delta": { "type": "input_json_delta", "partial_json": partial_json.to_string() }
                                    })));
                                    yield Ok(sse_event("content_block_stop", json!({
                                        "type": "content_block_stop",
                                        "index": index
                                    })));
                                }
                            }
                        }
                    }
                    Some("response.content_part.done") => {
                        if let Some(index) = text_index.take() {
                            yield Ok(sse_event("content_block_stop", json!({
                                "type": "content_block_stop",
                                "index": index
                            })));
                        }
                    }
                    Some("response.completed") | Some("response.incomplete") => {
                        if !sent_message_start {
                            sent_message_start = true;
                            yield Ok(message_start(&message_id, &model));
                        }
                        if let Some(index) = text_index.take() {
                            yield Ok(sse_event("content_block_stop", json!({
                                "type": "content_block_stop",
                                "index": index
                            })));
                        }
                        let response = data.get("response").unwrap_or(&data);
                        yield Ok(sse_event("message_delta", json!({
                            "type": "message_delta",
                            "delta": {
                                "stop_reason": stop_reason(response),
                                "stop_sequence": Value::Null
                            },
                            "usage": usage_from_responses(response.get("usage"))
                        })));
                        yield Ok(sse_event("message_stop", json!({ "type": "message_stop" })));
                    }
                    Some("error") => {
                        yield Ok(sse_event("error", json!({
                            "type": "error",
                            "error": data
                        })));
                    }
                    _ => {}
                }
            }
        }
    }
}

fn take_sse_block(buffer: &mut String) -> Option<String> {
    let index = buffer.find("\n\n")?;
    let block = buffer[..index].to_string();
    buffer.drain(..index + 2);
    Some(block)
}

fn parse_sse_block(block: &str) -> Option<(Option<String>, String)> {
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

fn message_start(message_id: &str, model: &str) -> Bytes {
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

fn sse_event(event: &str, data: Value) -> Bytes {
    Bytes::from(format!("event: {event}\ndata: {}\n\n", data))
}

fn usage_from_responses(usage: Option<&Value>) -> Value {
    let Some(usage) = usage else {
        return json!({ "input_tokens": 0, "output_tokens": 0 });
    };
    json!({
        "input_tokens": usage.get("input_tokens").and_then(Value::as_u64).unwrap_or(0),
        "output_tokens": usage.get("output_tokens").and_then(Value::as_u64).unwrap_or(0),
    })
}

fn stop_reason(response: &Value) -> &'static str {
    stop_reason_from_response(response)
}

fn stop_reason_from_response(response: &Value) -> &'static str {
    match response.get("status").and_then(Value::as_str) {
        Some("incomplete") => "max_tokens",
        _ => "end_turn",
    }
}

fn response_call_id(data: &Value) -> Option<String> {
    data.get("call_id")
        .or_else(|| data.get("item_id"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use futures::TryStreamExt;

    #[tokio::test]
    async fn converts_text_delta_stream() {
        let chunks = vec![Ok::<_, std::io::Error>(Bytes::from(
            "event: response.created\ndata: {\"response\":{\"id\":\"r1\",\"model\":\"gpt\"}}\n\n\
             event: response.output_text.delta\ndata: {\"delta\":\"hi\"}\n\n\
             event: response.completed\ndata: {\"response\":{\"status\":\"completed\",\"usage\":{\"input_tokens\":1,\"output_tokens\":2}}}\n\n",
        ))];
        let output = responses_sse_to_anthropic(stream::iter(chunks))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let text = String::from_utf8(output.concat().to_vec()).unwrap();
        assert!(text.contains("event: message_start"));
        assert!(text.contains("event: content_block_delta"));
        assert!(text.contains("\"text\":\"hi\""));
        assert!(text.contains("event: message_stop"));
    }
}
