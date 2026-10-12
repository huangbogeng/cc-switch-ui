use super::common::{message_start, parse_sse_block, sse_event, take_sse_block};
use crate::proxy::reasoning_bridge::anthropic_block_from_openai_reasoning_item;
use crate::proxy::responses_contract::{
    stop_reason, terminal_error, usage as usage_from_responses,
};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::collections::HashSet;

pub fn responses_sse_to_anthropic<E>(
    stream: impl Stream<Item = Result<Bytes, E>> + Send + 'static,
) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send
where
    E: std::error::Error + Send + 'static,
{
    async_stream::stream! {
        let mut buffer = Vec::new();
        let mut message_id = String::new();
        let mut model = String::new();
        let mut sent_message_start = false;
        let mut next_index: u32 = 0;
        let mut text_index: Option<u32> = None;
        let mut tool_indices: HashMap<String, u32> = HashMap::new();
        let mut pending_tool_args: HashMap<String, String> = HashMap::new();
        let mut reasoning_open: HashMap<String, (u32, String)> = HashMap::new();
        let mut reasoning_emitted = HashSet::new();
        let mut terminal = false;

        tokio::pin!(stream);

        while let Some(chunk) = stream.next().await {
            let bytes = match chunk {
                Ok(bytes) => bytes,
                Err(err) => {
                    log::error!(
                        "[Proxy] Upstream Responses stream chunk error: {}",
                        err
                    );
                    yield Err(std::io::Error::other(err.to_string()));
                    terminal = true;
                    break;
                }
            };

            buffer.extend_from_slice(&bytes);
            while let Some(block) = take_sse_block(&mut buffer) {
                let (event_name, data) = match parse_sse_block(&block) {
                    Some(parsed) => parsed,
                    None => continue,
                };
                let data: Value = match serde_json::from_str(&data) {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                match event_name.as_deref().or_else(|| data.get("type").and_then(Value::as_str)) {
                    Some("ping") => { yield Ok(sse_event("ping",json!({"type":"ping"}))); }
                    Some("response.reasoning_summary_text.delta" | "response.reasoning_text.delta" | "response.reasoning.delta") => {
                        if !sent_message_start { sent_message_start = true; yield Ok(message_start(&message_id,&model)); }
                        let key=reasoning_key(&data,&Value::Null);
                        let index=if let Some((index,_))=reasoning_open.get(&key) { *index } else {
                            let index=next_index; next_index+=1;
                            reasoning_open.insert(key.clone(),(index,String::new()));
                            yield Ok(sse_event("content_block_start",json!({"type":"content_block_start","index":index,"content_block":{"type":"thinking","thinking":""}})));
                            index
                        };
                        let delta=data.get("delta").or_else(||data.get("text")).and_then(Value::as_str).unwrap_or("");
                        reasoning_open.get_mut(&key).unwrap().1.push_str(delta);
                        yield Ok(sse_event("content_block_delta",json!({"type":"content_block_delta","index":index,"delta":{"type":"thinking_delta","thinking":delta}})));
                    }
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
                            if item.get("type").and_then(Value::as_str) == Some("reasoning") {
                                if !sent_message_start { sent_message_start = true; yield Ok(message_start(&message_id,&model)); }
                                for event in finish_reasoning(&data,item,&mut reasoning_open,&mut reasoning_emitted,&mut next_index) { yield Ok(event); }
                            }
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
                                    let partial_json = match serde_json::from_str::<Value>(&args) {
                                        Ok(value) => value,
                                        Err(error) => {
                                            yield Ok(sse_event("error",json!({"type":"error","error":{"type":"api_error","message":format!("Invalid Responses tool arguments: {error}")}})));
                                            terminal=true;
                                            break;
                                        }
                                    };
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
                        if let Some(error)=terminal_error(response) {
                            yield Ok(sse_event("error",json!({"type":"error","error":{"type":"api_error","message":error}})));
                            terminal=true; break;
                        }
                        for (output_index,item) in response.get("output").and_then(Value::as_array).into_iter().flatten().enumerate() {
                            if item["type"] == "reasoning" {
                                for event in finish_reasoning(&json!({"output_index":output_index}),item,&mut reasoning_open,&mut reasoning_emitted,&mut next_index) { yield Ok(event); }
                            }
                        }
                        for (_, (index,_)) in reasoning_open.drain() {
                            yield Ok(sse_event("content_block_stop",json!({"type":"content_block_stop","index":index})));
                        }
                        yield Ok(sse_event("message_delta", json!({
                            "type": "message_delta",
                            "delta": {
                                "stop_reason": stop_reason(response,!tool_indices.is_empty()),
                                "stop_sequence": Value::Null
                            },
                            "usage": usage_from_responses(response.get("usage"))
                        })));
                        yield Ok(sse_event("message_stop", json!({ "type": "message_stop" })));
                        terminal=true; break;
                    }
                    Some("error" | "response.failed" | "response.cancelled") => {
                        yield Ok(sse_event("error", json!({
                            "type": "error",
                            "error": {"type":"api_error","message":terminal_error(data.get("response").unwrap_or(&data)).unwrap_or_else(||data.to_string())}
                        })));
                        terminal=true; break;
                    }
                    _ => {}
                }
            }
            if terminal { break; }
        }
        if !terminal { yield Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof,"Responses stream ended without a terminal event")); }
    }
}

fn reasoning_key(data: &Value, item: &Value) -> String {
    item.get("id")
        .or_else(|| data.get("item_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| {
            format!(
                "output-{}",
                data.get("output_index")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            )
        })
}

fn finish_reasoning(
    data: &Value,
    item: &Value,
    open: &mut HashMap<String, (u32, String)>,
    emitted: &mut HashSet<String>,
    next_index: &mut u32,
) -> Vec<Bytes> {
    let key = reasoning_key(data, item);
    if !emitted.insert(key.clone()) {
        return vec![];
    }
    let existing = open.remove(&key);
    let mut item = item.clone();
    if let Some((_, text)) = &existing {
        if item
            .get("summary")
            .and_then(Value::as_array)
            .is_none_or(|parts| parts.is_empty())
            && !text.is_empty()
        {
            item["summary"] = json!([{"type":"summary_text","text":text}]);
        }
    }
    let Some(block) = anthropic_block_from_openai_reasoning_item(&item) else {
        return existing
            .map(|(index, _)| {
                vec![sse_event(
                    "content_block_stop",
                    json!({"type":"content_block_stop","index":index}),
                )]
            })
            .unwrap_or_default();
    };
    let mut events = Vec::new();
    let (index, prior) = if let Some(existing) = existing {
        existing
    } else {
        let index = *next_index;
        *next_index += 1;
        let initial = if block["type"] == "thinking" {
            json!({"type":"thinking","thinking":""})
        } else {
            block.clone()
        };
        events.push(sse_event(
            "content_block_start",
            json!({"type":"content_block_start","index":index,"content_block":initial}),
        ));
        (index, String::new())
    };
    if let Some(text) = block.get("thinking").and_then(Value::as_str) {
        let delta = text
            .strip_prefix(&prior)
            .unwrap_or(if prior.is_empty() { text } else { "" });
        if !delta.is_empty() {
            events.push(sse_event("content_block_delta",json!({"type":"content_block_delta","index":index,"delta":{"type":"thinking_delta","thinking":delta}})));
        }
    }
    if let Some(signature) = block.get("signature") {
        events.push(sse_event("content_block_delta",json!({"type":"content_block_delta","index":index,"delta":{"type":"signature_delta","signature":signature}})));
    }
    events.push(sse_event(
        "content_block_stop",
        json!({"type":"content_block_stop","index":index}),
    ));
    events
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
    async fn reasoning_signature_replays_and_utf8_survives_single_byte_chunks() {
        use crate::proxy::reasoning_bridge::decode_openai_reasoning_item;
        let item = json!({"id":"rs","type":"reasoning","summary":[{"type":"summary_text","text":"先检查"}],"encrypted_content":"opaque","status":"completed"});
        let events = [
            (
                "response.created",
                json!({"response":{"id":"r","model":"gpt-5"}}),
            ),
            (
                "response.reasoning_summary_text.delta",
                json!({"item_id":"rs","delta":"先检查"}),
            ),
            ("response.output_item.done", json!({"item":item})),
            (
                "response.completed",
                json!({"response":{"status":"completed","output":[item]}}),
            ),
            ("response.failed", json!({})),
        ]
        .into_iter()
        .map(|(name, data)| format!("event: {name}\r\ndata: {data}\r\n\r\n"))
        .collect::<String>();
        let chunks = events
            .into_bytes()
            .into_iter()
            .map(|byte| Ok::<_, std::io::Error>(Bytes::from(vec![byte])))
            .collect::<Vec<_>>();
        let output = responses_sse_to_anthropic(stream::iter(chunks))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let events = String::from_utf8(output.concat()).unwrap();
        assert!(events.contains("先检查"));
        assert_eq!(events.matches("thinking_delta").count(), 1);
        assert_eq!(events.matches("signature_delta").count(), 1);
        assert_eq!(events.matches("event: message_stop").count(), 1);
        assert!(!events.contains("event: error"));
        let signature = events
            .split("\n\n")
            .filter_map(parse_sse_block)
            .filter_map(|(_, data)| serde_json::from_str::<Value>(&data).ok())
            .find_map(|data| {
                data.pointer("/delta/signature")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap();
        let replay = decode_openai_reasoning_item(&signature).unwrap();
        assert_eq!(replay["encrypted_content"], "opaque");
        assert!(replay.get("status").is_none());
    }

    #[tokio::test]
    async fn failures_and_disconnects_never_emit_success() {
        for status in ["failed", "cancelled"] {
            let event=format!("event: response.{status}\ndata: {{\"response\":{{\"status\":\"{status}\",\"error\":{{\"message\":\"unavailable\"}}}}}}\n\n");
            let output = responses_sse_to_anthropic(stream::iter([Ok::<_, std::io::Error>(
                Bytes::from(event),
            )]))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
            let text = String::from_utf8(output.concat()).unwrap();
            assert!(text.contains("event: error"));
            assert!(!text.contains("event: message_stop"));
        }
        let output = responses_sse_to_anthropic(stream::iter([Ok::<_, std::io::Error>(
            Bytes::from_static(b"event: response.created\ndata: {}\n\n"),
        )]))
        .try_collect::<Vec<_>>()
        .await
        .unwrap_err();
        assert_eq!(output.kind(), std::io::ErrorKind::UnexpectedEof);
        let chunks = [
            Err(std::io::Error::other("connection reset")),
            Ok(Bytes::from_static(
                b"event: response.completed\ndata: {}\n\n",
            )),
        ];
        let output = responses_sse_to_anthropic(stream::iter(chunks))
            .collect::<Vec<_>>()
            .await;
        assert_eq!(output.len(), 1);
        assert!(output[0].is_err());
    }

    #[tokio::test]
    async fn malformed_tool_arguments_fail_instead_of_becoming_empty_input() {
        let item = json!({"id":"item","type":"function_call","call_id":"call","name":"Read","arguments":"{broken"});
        // Some compatible services put the event type in data, without event:.
        let events = [
            json!({"type":"response.output_item.added","item":item}),
            json!({"type":"response.output_item.done","item":item}),
            json!({"type":"response.completed","response":{"status":"completed"}}),
        ]
        .into_iter()
        .map(|data| format!("data: {data}\n\n"))
        .collect::<String>();
        let output = responses_sse_to_anthropic(stream::iter([Ok::<_, std::io::Error>(
            Bytes::from(events),
        )]))
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
        let text = String::from_utf8(output.concat()).unwrap();
        assert!(text.contains("Invalid Responses tool arguments"));
        assert!(!text.contains("input_json_delta"));
        assert!(!text.contains("event: message_stop"));
    }

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

    #[tokio::test]
    async fn finishes_function_calls_with_tool_use_stop_reason() {
        let item = json!({"id":"item_1","type":"function_call","call_id":"call_1","name":"Read","arguments":"{\"path\":\"fixture\"}"});
        let events = [
            (
                "response.created",
                json!({"response":{"id":"r","model":"test"}}),
            ),
            ("response.output_item.added", json!({"item":item})),
            ("response.output_item.done", json!({"item":item})),
            (
                "response.completed",
                json!({"response":{"status":"completed"}}),
            ),
        ]
        .into_iter()
        .map(|(name, data)| format!("event: {name}\ndata: {data}\n\n"))
        .collect::<String>();
        let output = responses_sse_to_anthropic(stream::iter([Ok::<_, std::io::Error>(
            Bytes::from(events),
        )]))
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
        let text = String::from_utf8(output.concat().to_vec()).unwrap();
        assert!(text.contains("\"stop_reason\":\"tool_use\""));
        assert!(text.contains("\"id\":\"call_1\""));
        assert!(text.contains("event: content_block_stop"));
        assert_eq!(text.matches("event: message_stop").count(), 1);
    }
}
