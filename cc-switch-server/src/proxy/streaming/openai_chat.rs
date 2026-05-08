//! OpenAI Chat SSE to Anthropic Messages SSE conversion.

use super::common::{message_start, parse_sse_block, sse_event, take_sse_block};
use super::finalization::{
    close_text_block_if_needed, close_thinking_block_if_needed, message_stop_event,
};
use super::tool_blocks::{close_open_tool_blocks, guarded_openai_finish_reason, ToolBlockState};
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
        // SSE payload may split logical events across chunks; keep a rolling buffer.
        let mut buffer = String::new();
        let mut message_id = String::new();
        let mut model = String::new();
        let mut sent_message_start = false;
        let mut sent_text_start = false;
        let mut sent_text_stop = false;
        let mut text_block_index: Option<u32> = None;
        let mut thinking_block_index: Option<u32> = None;
        let mut sent_message_stop = false;
        let mut sent_usage = false;
        let mut latest_usage: Option<OpenAIChatStreamUsage> = None;
        let mut next_content_index: u32 = 0;
        let mut tool_blocks_by_index: HashMap<u32, ToolBlockState> = HashMap::new();
        let mut open_tool_block_indices: Vec<u32> = Vec::new();
        let mut valid_closed_tool_blocks: u32 = 0;

        tokio::pin!(stream);

        while let Some(chunk) = stream.next().await {
            let bytes = match chunk {
                Ok(bytes) => bytes,
                Err(err) => {
                    log::error!(
                        "[Proxy] Upstream OpenAI-chat stream chunk error: {}",
                        err
                    );
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
                    // OpenAI chat may end without explicit tool block stop events.
                    // We force-close any open tool blocks to keep Anthropic event ordering valid.
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
                    if let Some(event) = close_text_block_if_needed(
                        sent_text_start,
                        &mut sent_text_stop,
                        text_block_index,
                    ) {
                        yield Ok(event);
                    }
                    if let Some(event) = close_thinking_block_if_needed(&mut thinking_block_index)
                    {
                        yield Ok(event);
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
                    for event in message_stop_event("end_turn", json!({ "output_tokens": 0 })) {
                        yield Ok(event);
                    }
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

                if let Some(reasoning) = choice
                    .get("delta")
                    .and_then(|delta| delta.get("reasoning_content"))
                    .and_then(Value::as_str)
                {
                    for event in handle_reasoning_delta(
                        reasoning,
                        &mut thinking_block_index,
                        &mut next_content_index,
                    ) {
                        yield Ok(event);
                    }
                }

                if let Some(content) = choice
                    .get("delta")
                    .and_then(|delta| delta.get("content"))
                    .and_then(Value::as_str)
                {
                    for event in handle_text_delta(
                        content,
                        &mut sent_text_start,
                        &mut sent_text_stop,
                        &mut text_block_index,
                        &mut next_content_index,
                    ) {
                        yield Ok(event);
                    }
                }

                if let Some(tool_calls) = choice
                    .get("delta")
                    .and_then(|delta| delta.get("tool_calls"))
                    .and_then(Value::as_array)
                {
                    for event in handle_tool_calls(
                        tool_calls,
                        &mut sent_text_start,
                        &mut sent_text_stop,
                        text_block_index,
                        &mut next_content_index,
                        &mut tool_blocks_by_index,
                        &mut open_tool_block_indices,
                    ) {
                        yield Ok(event);
                    }
                }

                let finish_reason = choice.get("finish_reason").and_then(Value::as_str);
                if finish_reason.is_some() {
                    if sent_message_stop {
                        continue;
                    }
                    if let Some(event) = close_text_block_if_needed(
                        sent_text_start,
                        &mut sent_text_stop,
                        text_block_index,
                    ) {
                        yield Ok(event);
                    }
                    if let Some(event) = close_thinking_block_if_needed(&mut thinking_block_index)
                    {
                        yield Ok(event);
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
                    // Guard tool-use stop reason to avoid emitting invalid Anthropic tool_use states.
                    let stop_reason = guarded_openai_finish_reason(
                        finish_reason,
                        valid_closed_tool_blocks > 0,
                        &model,
                        &message_id,
                    );
                    for event in message_stop_event(stop_reason, openai_usage(data.get("usage"))) {
                        yield Ok(event);
                    }
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

fn openai_usage(usage: Option<&Value>) -> Value {
    let Some(usage) = usage else {
        return json!({ "output_tokens": 0 });
    };
    json!({
        "input_tokens": token_usage_value(usage, &["prompt_tokens", "input_tokens"]).unwrap_or(0),
        "output_tokens": token_usage_value(usage, &["completion_tokens", "output_tokens"]).unwrap_or(0),
    })
}

fn handle_reasoning_delta(
    reasoning: &str,
    thinking_block_index: &mut Option<u32>,
    next_content_index: &mut u32,
) -> Vec<Bytes> {
    if reasoning.is_empty() {
        return Vec::new();
    }

    let mut events = Vec::new();
    let index = if let Some(index) = *thinking_block_index {
        index
    } else {
        let index = *next_content_index;
        *next_content_index += 1;
        *thinking_block_index = Some(index);
        events.push(sse_event(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": index,
                "content_block": { "type": "thinking", "thinking": "" }
            }),
        ));
        index
    };

    events.push(sse_event(
        "content_block_delta",
        json!({
            "type": "content_block_delta",
            "index": index,
            "delta": { "type": "thinking_delta", "thinking": reasoning }
        }),
    ));
    events
}

fn handle_text_delta(
    content: &str,
    sent_text_start: &mut bool,
    sent_text_stop: &mut bool,
    text_block_index: &mut Option<u32>,
    next_content_index: &mut u32,
) -> Vec<Bytes> {
    let mut events = Vec::new();
    if !*sent_text_start {
        *sent_text_start = true;
        *sent_text_stop = false;
        let index = *next_content_index;
        *next_content_index += 1;
        *text_block_index = Some(index);
        events.push(sse_event(
            "content_block_start",
            json!({
                "type": "content_block_start",
                "index": index,
                "content_block": { "type": "text", "text": "" }
            }),
        ));
    }

    let index = text_block_index.unwrap_or(0);
    events.push(sse_event(
        "content_block_delta",
        json!({
            "type": "content_block_delta",
            "index": index,
            "delta": { "type": "text_delta", "text": content }
        }),
    ));
    events
}

fn handle_tool_calls(
    tool_calls: &[Value],
    sent_text_start: &mut bool,
    sent_text_stop: &mut bool,
    text_block_index: Option<u32>,
    next_content_index: &mut u32,
    tool_blocks_by_index: &mut HashMap<u32, ToolBlockState>,
    open_tool_block_indices: &mut Vec<u32>,
) -> Vec<Bytes> {
    let mut events = Vec::new();
    if *sent_text_start && !*sent_text_stop {
        *sent_text_stop = true;
        let index = text_block_index.unwrap_or(0);
        events.push(sse_event(
            "content_block_stop",
            json!({
                "type": "content_block_stop",
                "index": index
            }),
        ));
    }

    for tool_call in tool_calls {
        let tool_index = tool_call.get("index").and_then(Value::as_u64).unwrap_or(0) as u32;
        let state = tool_blocks_by_index.entry(tool_index).or_insert_with(|| {
            let idx = *next_content_index;
            *next_content_index += 1;
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

        // Anthropic requires id+name at block start. Delay start until both are known.
        let should_start = !state.started && !state.id.is_empty() && !state.name.is_empty();
        if should_start {
            state.started = true;
            let index = state.anthropic_index;
            events.push(sse_event(
                "content_block_start",
                json!({
                    "type": "content_block_start",
                    "index": index,
                    "content_block": {
                        "type": "tool_use",
                        "id": state.id,
                        "name": state.name
                    }
                }),
            ));
            open_tool_block_indices.push(tool_index);
            if !state.pending_args.is_empty() {
                let args = std::mem::take(&mut state.pending_args);
                events.push(sse_event(
                    "content_block_delta",
                    json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": { "type": "input_json_delta", "partial_json": args }
                    }),
                ));
            }
        }

        if let Some(arguments) = tool_call
            .get("function")
            .and_then(|f| f.get("arguments"))
            .and_then(Value::as_str)
        {
            state.all_arguments.push_str(arguments);
            if state.started {
                events.push(sse_event(
                    "content_block_delta",
                    json!({
                        "type": "content_block_delta",
                        "index": state.anthropic_index,
                        "delta": { "type": "input_json_delta", "partial_json": arguments }
                    }),
                ));
            } else {
                state.pending_args.push_str(arguments);
            }
        }
    }

    events
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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use futures::TryStreamExt;

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
    async fn converts_openai_chat_reasoning_content_to_thinking_block() {
        let chunks = vec![Ok::<_, std::io::Error>(Bytes::from(
            "data: {\"id\":\"c2\",\"model\":\"deepseek-v4-pro\",\"choices\":[{\"delta\":{\"reasoning_content\":\"think-a\"}}]}\n\n\
             data: {\"id\":\"c2\",\"model\":\"deepseek-v4-pro\",\"choices\":[{\"delta\":{\"reasoning_content\":\"think-b\"}}]}\n\n\
             data: {\"id\":\"c2\",\"model\":\"deepseek-v4-pro\",\"choices\":[{\"delta\":{\"content\":\"final\"}}]}\n\n\
             data: {\"id\":\"c2\",\"model\":\"deepseek-v4-pro\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n\
             data: [DONE]\n\n",
        ))];

        let output = openai_chat_sse_to_anthropic(stream::iter(chunks))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();
        let text = String::from_utf8(output.concat().to_vec()).unwrap();

        assert!(text.contains("\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}"));
        assert!(text.contains("\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"think-a\"}"));
        assert!(text.contains("\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"think-b\"}"));
        assert!(text.contains("\"content_block\":{\"type\":\"text\",\"text\":\"\"}"));
        assert!(text.contains("\"delta\":{\"type\":\"text_delta\",\"text\":\"final\"}"));
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

        assert!(text.contains("\"type\":\"tool_use\""));
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
