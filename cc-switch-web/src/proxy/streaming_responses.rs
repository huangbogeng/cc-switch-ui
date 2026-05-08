//! OpenAI Responses SSE to Anthropic Messages SSE conversion.

use bytes::Bytes;
use futures::{Stream, StreamExt};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAIChatStreamUsage {
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

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
                    yield Err(std::io::Error::other(err.to_string()));
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

#[cfg(test)]
pub fn openai_chat_sse_to_anthropic<E>(
    stream: impl Stream<Item = Result<Bytes, E>> + Send + 'static,
) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send
where
    E: std::error::Error + Send + 'static,
{
    openai_chat_sse_to_anthropic_with_usage(stream, |_| {})
}

pub fn openai_chat_sse_to_anthropic_with_usage<E, F>(
    stream: impl Stream<Item = Result<Bytes, E>> + Send + 'static,
    on_usage: F,
) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send
where
    E: std::error::Error + Send + 'static,
    F: Fn(OpenAIChatStreamUsage) + Send + 'static,
{
    async_stream::stream! {
        let mut buffer = String::new();
        let mut message_id = String::new();
        let mut model = String::new();
        let mut sent_message_start = false;
        let mut sent_text_start = false;
        let mut sent_text_stop = false;
        let mut sent_message_stop = false;
        let mut sent_usage = false;
        let mut latest_usage: Option<OpenAIChatStreamUsage> = None;
        let mut next_content_index: u32 = 1;
        let mut tool_blocks_by_index: HashMap<u32, ToolBlockState> = HashMap::new();
        let mut open_tool_block_indices: Vec<u32> = Vec::new();
        let mut valid_closed_tool_blocks: u32 = 0;

        tokio::pin!(stream);

        while let Some(chunk) = stream.next().await {
            let bytes = match chunk {
                Ok(bytes) => bytes,
                Err(err) => {
                    yield Err(std::io::Error::other(err.to_string()));
                    continue;
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(block) = take_sse_block(&mut buffer) {
                let Some((_event_name, data)) = parse_sse_block(&block) else {
                    continue;
                };
                if data.trim() == "[DONE]" {
                    if !sent_usage {
                        if let Some(usage) = latest_usage.take() {
                            on_usage(usage);
                            sent_usage = true;
                        }
                    }
                    let closed_tool_block_indices = close_open_tool_blocks(
                        &mut open_tool_block_indices,
                        &mut tool_blocks_by_index,
                        &mut valid_closed_tool_blocks,
                        &model,
                        &message_id,
                        "invalid_tool_block_on_done",
                    );
                    if sent_message_stop {
                        continue;
                    }
                    if sent_text_start && !sent_text_stop {
                        sent_text_stop = true;
                        yield Ok(sse_event("content_block_stop", json!({
                            "type": "content_block_stop",
                            "index": 0
                        })));
                    }
                    for index in closed_tool_block_indices {
                        yield Ok(sse_event("content_block_stop", json!({
                            "type": "content_block_stop",
                            "index": index
                        })));
                    }
                    if !sent_message_start {
                        yield Ok(message_start(&message_id, &model));
                    }
                    yield Ok(sse_event("message_delta", json!({
                        "type": "message_delta",
                        "delta": {
                            "stop_reason": "end_turn",
                            "stop_sequence": Value::Null
                        },
                        "usage": { "output_tokens": 0 }
                    })));
                    yield Ok(sse_event("message_stop", json!({ "type": "message_stop" })));
                    sent_message_stop = true;
                    continue;
                }

                let data: Value = match serde_json::from_str(&data) {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                if message_id.is_empty() {
                    if let Some(id) = data.get("id").and_then(Value::as_str) {
                        message_id = id.to_string();
                    }
                }
                if model.is_empty() {
                    if let Some(response_model) = data.get("model").and_then(Value::as_str) {
                        model = response_model.to_string();
                    }
                }
                if let Some(usage) = openai_chat_stream_usage(data.get("usage"), &model) {
                    latest_usage = Some(usage);
                }
                if !sent_message_start {
                    sent_message_start = true;
                    yield Ok(message_start(&message_id, &model));
                }

                let Some(choice) = data
                    .get("choices")
                    .and_then(Value::as_array)
                    .and_then(|choices| choices.first()) else {
                    continue;
                };
                if sent_message_stop {
                    continue;
                }

                if let Some(content) = choice
                    .get("delta")
                    .and_then(|delta| delta.get("content"))
                    .and_then(Value::as_str)
                {
                    if !sent_text_start {
                        sent_text_start = true;
                        sent_text_stop = false;
                        yield Ok(sse_event("content_block_start", json!({
                            "type": "content_block_start",
                            "index": 0,
                            "content_block": { "type": "text", "text": "" }
                        })));
                    }
                    yield Ok(sse_event("content_block_delta", json!({
                        "type": "content_block_delta",
                        "index": 0,
                        "delta": { "type": "text_delta", "text": content }
                    })));
                }

                if let Some(tool_calls) = choice
                    .get("delta")
                    .and_then(|delta| delta.get("tool_calls"))
                    .and_then(Value::as_array)
                {
                    if sent_text_start && !sent_text_stop {
                        sent_text_stop = true;
                        yield Ok(sse_event("content_block_stop", json!({
                            "type": "content_block_stop",
                            "index": 0
                        })));
                    }
                    for tool_call in tool_calls {
                        let tool_index = tool_call
                            .get("index")
                            .and_then(Value::as_u64)
                            .unwrap_or(0) as u32;

                        let state = tool_blocks_by_index
                            .entry(tool_index)
                            .or_insert_with(|| {
                                let idx = next_content_index;
                                next_content_index += 1;
                                ToolBlockState {
                                    anthropic_index: idx,
                                    id: String::new(),
                                    name: String::new(),
                                    started: false,
                                    pending_args: String::new(),
                                    all_arguments: String::new(),
                                }
                            });

                        if let Some(id) = tool_call.get("id").and_then(Value::as_str) {
                            state.id = id.to_string();
                        }
                        if let Some(name) = tool_call
                            .get("function")
                            .and_then(|f| f.get("name"))
                            .and_then(Value::as_str)
                        {
                            state.name = name.to_string();
                        }

                        let should_start = !state.started && !state.id.is_empty() && !state.name.is_empty();
                        if should_start {
                            state.started = true;
                            let index = state.anthropic_index;
                            yield Ok(sse_event("content_block_start", json!({
                                "type": "content_block_start",
                                "index": index,
                                "content_block": {
                                    "type": "tool_use",
                                    "id": state.id,
                                    "name": state.name
                                }
                            })));
                            open_tool_block_indices.push(tool_index);
                            if !state.pending_args.is_empty() {
                                let args = std::mem::take(&mut state.pending_args);
                                yield Ok(sse_event("content_block_delta", json!({
                                    "type": "content_block_delta",
                                    "index": index,
                                    "delta": { "type": "input_json_delta", "partial_json": args }
                                })));
                            }
                        }

                        if let Some(arguments) = tool_call
                            .get("function")
                            .and_then(|f| f.get("arguments"))
                            .and_then(Value::as_str)
                        {
                            state.all_arguments.push_str(arguments);
                            if state.started {
                                yield Ok(sse_event("content_block_delta", json!({
                                    "type": "content_block_delta",
                                    "index": state.anthropic_index,
                                    "delta": { "type": "input_json_delta", "partial_json": arguments }
                                })));
                            } else {
                                state.pending_args.push_str(arguments);
                            }
                        }
                    }
                }

                let finish_reason = choice.get("finish_reason").and_then(Value::as_str);
                if finish_reason.is_some() {
                    if sent_message_stop {
                        continue;
                    }
                    if sent_text_start && !sent_text_stop {
                        sent_text_stop = true;
                        yield Ok(sse_event("content_block_stop", json!({
                            "type": "content_block_stop",
                            "index": 0
                        })));
                    }
                    let closed_tool_block_indices = close_open_tool_blocks(
                        &mut open_tool_block_indices,
                        &mut tool_blocks_by_index,
                        &mut valid_closed_tool_blocks,
                        &model,
                        &message_id,
                        "invalid_tool_block_on_finish",
                    );
                    for index in closed_tool_block_indices {
                        yield Ok(sse_event("content_block_stop", json!({
                            "type": "content_block_stop",
                            "index": index
                        })));
                    }
                    let stop_reason = guarded_openai_finish_reason(
                        finish_reason,
                        valid_closed_tool_blocks > 0,
                        &model,
                        &message_id,
                    );
                    yield Ok(sse_event("message_delta", json!({
                        "type": "message_delta",
                        "delta": {
                            "stop_reason": stop_reason,
                            "stop_sequence": Value::Null
                        },
                        "usage": openai_usage(data.get("usage"))
                    })));
                    yield Ok(sse_event("message_stop", json!({ "type": "message_stop" })));
                    sent_message_stop = true;
                }
            }
        }

        if !sent_usage {
            if let Some(usage) = latest_usage.take() {
                on_usage(usage);
            }
        }
    }
}

#[derive(Debug, Default)]
struct ToolBlockState {
    anthropic_index: u32,
    id: String,
    name: String,
    started: bool,
    pending_args: String,
    all_arguments: String,
}

fn close_open_tool_blocks(
    open_tool_block_indices: &mut Vec<u32>,
    tool_blocks_by_index: &mut HashMap<u32, ToolBlockState>,
    valid_closed_tool_blocks: &mut u32,
    model: &str,
    message_id: &str,
    warn_reason: &str,
) -> Vec<u32> {
    let mut closed_indices = Vec::with_capacity(open_tool_block_indices.len());
    for tool_index in open_tool_block_indices.drain(..) {
        let Some(state) = tool_blocks_by_index.remove(&tool_index) else {
            continue;
        };
        if is_valid_tool_block(&state) {
            *valid_closed_tool_blocks += 1;
        } else {
            log::warn!(
                "tool_use_downgrade reason={} model={} message_id={} tool_index={}",
                warn_reason,
                model,
                message_id,
                tool_index
            );
        }
        closed_indices.push(state.anthropic_index);
    }
    closed_indices
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

fn openai_usage(usage: Option<&Value>) -> Value {
    let Some(usage) = usage else {
        return json!({ "output_tokens": 0 });
    };
    json!({
        "input_tokens": token_usage_value(usage, &["prompt_tokens", "input_tokens"]).unwrap_or(0),
        "output_tokens": token_usage_value(usage, &["completion_tokens", "output_tokens"]).unwrap_or(0),
    })
}

fn openai_chat_stream_usage(usage: Option<&Value>, model: &str) -> Option<OpenAIChatStreamUsage> {
    let usage = usage?;
    let input_tokens = token_usage_value(usage, &["prompt_tokens", "input_tokens"])?;
    let output_tokens = token_usage_value(usage, &["completion_tokens", "output_tokens"])?;
    Some(OpenAIChatStreamUsage {
        model: model.to_string(),
        input_tokens,
        output_tokens,
    })
}

fn token_usage_value(usage: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter()
        .find_map(|key| usage.get(*key).and_then(Value::as_i64))
}

fn openai_finish_reason(reason: Option<&str>) -> &'static str {
    match reason {
        Some("length") => "max_tokens",
        Some("tool_calls") | Some("function_call") => "tool_use",
        _ => "end_turn",
    }
}

fn guarded_openai_finish_reason(
    reason: Option<&str>,
    has_valid_closed_tool_block: bool,
    model: &str,
    message_id: &str,
) -> &'static str {
    if matches!(reason, Some("tool_calls") | Some("function_call")) && !has_valid_closed_tool_block
    {
        log::warn!(
            "tool_use_downgrade reason=missing_valid_closed_tool_block finish_reason={} model={} message_id={}",
            reason.unwrap_or_default(),
            model,
            message_id
        );
        return "end_turn";
    }
    openai_finish_reason(reason)
}

fn is_valid_tool_block(state: &ToolBlockState) -> bool {
    if !state.started || state.id.is_empty() || state.name.is_empty() {
        return false;
    }
    let args = state.all_arguments.trim();
    args.is_empty() || serde_json::from_str::<Value>(args).is_ok()
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

    #[tokio::test]
    async fn converts_openai_chat_delta_stream_once() {
        let chunks = vec![Ok::<_, std::io::Error>(Bytes::from(
            "data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2}}\n\n\
             data: [DONE]\n\n",
        ))];
        let output = openai_chat_sse_to_anthropic(stream::iter(chunks))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let text = String::from_utf8(output.concat().to_vec()).unwrap();

        assert!(text.contains("event: message_start"));
        assert!(text.contains("\"text\":\"hi\""));
        assert_eq!(text.matches("event: message_stop").count(), 1);
        assert_eq!(text.matches("event: content_block_stop").count(), 1);
    }

    #[tokio::test]
    async fn captures_openai_chat_stream_usage_from_usage_chunk() {
        let chunks = vec![Ok::<_, std::io::Error>(Bytes::from(
            "data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{}}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":5}}\n\n\
             data: [DONE]\n\n",
        ))];
        let usage = std::sync::Arc::new(std::sync::Mutex::new(None));
        let captured = usage.clone();

        let output = openai_chat_sse_to_anthropic_with_usage(stream::iter(chunks), move |record| {
            *captured.lock().unwrap() = Some(record);
        })
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
        let text = String::from_utf8(output.concat().to_vec()).unwrap();
        let usage = usage.lock().unwrap().clone().unwrap();

        assert!(text.contains("\"text\":\"hi\""));
        assert_eq!(usage.model, "MiniMax-M2.7");
        assert_eq!(usage.input_tokens, 3);
        assert_eq!(usage.output_tokens, 5);
    }

    #[tokio::test]
    async fn captures_openai_chat_stream_usage_with_input_output_aliases() {
        let chunks = vec![Ok::<_, std::io::Error>(Bytes::from(
            "data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[],\"usage\":{\"input_tokens\":7,\"output_tokens\":11}}\n\n",
        ))];
        let usage = std::sync::Arc::new(std::sync::Mutex::new(None));
        let captured = usage.clone();

        openai_chat_sse_to_anthropic_with_usage(stream::iter(chunks), move |record| {
            *captured.lock().unwrap() = Some(record);
        })
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
        let usage = usage.lock().unwrap().clone().unwrap();

        assert_eq!(usage.input_tokens, 7);
        assert_eq!(usage.output_tokens, 11);
    }

    #[tokio::test]
    async fn converts_openai_chat_stream_tool_calls() {
        let chunks = vec![Ok::<_, std::io::Error>(Bytes::from(
            "data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"lookup\"}}]}}]}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{\\\"q\\\":\\\"hi\\\"}\"}}]}}]}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{},\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":5}}\n\n\
             data: [DONE]\n\n",
        ))];

        let output = openai_chat_sse_to_anthropic(stream::iter(chunks))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let text = String::from_utf8(output.concat().to_vec()).unwrap();

        assert!(text.contains("\"type\":\"tool_use\""));
        assert!(text.contains("\"name\":\"lookup\""));
        assert!(text.contains("\"partial_json\":\"{\\\"q\\\":\\\"hi\\\"}\""));
        assert!(text.contains("\"stop_reason\":\"tool_use\""));
    }

    #[tokio::test]
    async fn downgrades_tool_stop_reason_without_valid_tool_block() {
        let chunks = vec![Ok::<_, std::io::Error>(Bytes::from(
            "data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"type\":\"function\",\"function\":{\"arguments\":\"{\\\"q\\\":\\\"hi\\\"\"}}]}}]}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{},\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":5}}\n\n\
             data: [DONE]\n\n",
        ))];

        let output = openai_chat_sse_to_anthropic(stream::iter(chunks))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let text = String::from_utf8(output.concat().to_vec()).unwrap();

        assert!(!text.contains("\"type\":\"tool_use\""));
        assert!(text.contains("\"stop_reason\":\"end_turn\""));
    }

    #[tokio::test]
    async fn keeps_tool_stop_reason_with_valid_closed_tool_block() {
        let chunks = vec![Ok::<_, std::io::Error>(Bytes::from(
            "data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"lookup\"}}]}}]}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{\\\"q\\\":\\\"hi\\\"}\"}}]}}]}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{},\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":5}}\n\n\
             data: [DONE]\n\n",
        ))];

        let output = openai_chat_sse_to_anthropic(stream::iter(chunks))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let text = String::from_utf8(output.concat().to_vec()).unwrap();

        assert!(text.contains("\"type\":\"tool_use\""));
        assert!(text.contains("\"stop_reason\":\"tool_use\""));
    }

    #[tokio::test]
    async fn closes_open_tool_blocks_on_done_without_duplicate_message_stop() {
        let chunks = vec![Ok::<_, std::io::Error>(Bytes::from(
            "data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"lookup\"}}]}}]}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{\\\"q\\\":\\\"hi\\\"}\"}}]}}]}\n\n\
             data: [DONE]\n\n\
             data: [DONE]\n\n",
        ))];

        let output = openai_chat_sse_to_anthropic(stream::iter(chunks))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let text = String::from_utf8(output.concat().to_vec()).unwrap();

        assert!(text.contains("\"index\":1"));
        assert_eq!(text.matches("event: content_block_stop").count(), 1);
        assert_eq!(text.matches("event: message_stop").count(), 1);
    }

    #[tokio::test]
    async fn ignores_second_finish_reason_chunk() {
        let chunks = vec![Ok::<_, std::io::Error>(Bytes::from(
            "data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2}}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2}}\n\n\
             data: [DONE]\n\n",
        ))];

        let output = openai_chat_sse_to_anthropic(stream::iter(chunks))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let text = String::from_utf8(output.concat().to_vec()).unwrap();

        assert_eq!(text.matches("event: message_delta").count(), 1);
        assert_eq!(text.matches("event: message_stop").count(), 1);
    }

    #[tokio::test]
    async fn ignores_tool_calls_after_message_stop_before_done() {
        let chunks = vec![Ok::<_, std::io::Error>(Bytes::from(
            "data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":2}}\n\n\
             data: {\"id\":\"c1\",\"model\":\"MiniMax-M2.7\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"type\":\"function\",\"function\":{\"name\":\"lookup\",\"arguments\":\"{\\\"q\\\":\\\"late\\\"}\"}}]}}]}\n\n\
             data: [DONE]\n\n",
        ))];

        let output = openai_chat_sse_to_anthropic(stream::iter(chunks))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let text = String::from_utf8(output.concat().to_vec()).unwrap();

        assert_eq!(text.matches("event: message_stop").count(), 1);
        assert!(!text.contains("\"type\":\"tool_use\""));
        assert!(!text.contains("\"partial_json\":\"{\\\"q\\\":\\\"late\\\"}\""));
    }
}
