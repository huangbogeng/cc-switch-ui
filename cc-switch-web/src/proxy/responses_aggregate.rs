//! OpenAI Responses SSE aggregation for non-streaming Anthropic clients.

use serde_json::{json, Value};
use std::collections::HashMap;

pub fn responses_sse_to_anthropic_message(bytes: &[u8]) -> Result<Value, String> {
    let text = String::from_utf8_lossy(bytes);
    let mut message_id = String::new();
    let mut model = String::new();
    let mut text_parts = Vec::new();
    let mut usage = json!({ "input_tokens": 0, "output_tokens": 0 });
    let mut stop_reason = "end_turn";
    let mut tool_calls: HashMap<String, ToolCallAccumulator> = HashMap::new();
    let mut tool_aliases: HashMap<String, String> = HashMap::new();

    for block in text.split("\n\n") {
        let Some((event_name, data)) = parse_sse_block(block) else {
            continue;
        };
        let Ok(data) = serde_json::from_str::<Value>(&data) else {
            continue;
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
            }
            Some("response.output_text.delta") | Some("response.refusal.delta") => {
                if let Some(delta) = data.get("delta").and_then(Value::as_str) {
                    text_parts.push(delta.to_string());
                }
            }
            Some("response.output_item.added") => {
                if let Some(item) = data.get("item") {
                    if item.get("type").and_then(Value::as_str) == Some("function_call") {
                        remember_tool_call(&mut tool_calls, &mut tool_aliases, item);
                    }
                }
            }
            Some("response.function_call_arguments.delta") => {
                if let Some(id) = response_call_id(&data) {
                    let delta = data
                        .get("delta")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let key = canonical_tool_key(&tool_aliases, &id);
                    tool_calls
                        .entry(key)
                        .or_insert_with(ToolCallAccumulator::default)
                        .arguments
                        .push_str(delta);
                }
            }
            Some("response.output_item.done") => {
                if let Some(item) = data.get("item") {
                    if item.get("type").and_then(Value::as_str) == Some("function_call") {
                        finish_tool_call(&mut tool_calls, &mut tool_aliases, item);
                    }
                }
            }
            Some("response.completed") | Some("response.incomplete") => {
                let response = data.get("response").unwrap_or(&data);
                stop_reason = stop_reason_from_response(response);
                usage = usage_from_responses(response.get("usage"));
                if let Some(id) = response.get("id").and_then(Value::as_str) {
                    message_id = id.to_string();
                }
                if let Some(response_model) = response.get("model").and_then(Value::as_str) {
                    model = response_model.to_string();
                }
            }
            Some("error") => return Err(data.to_string()),
            _ => {}
        }
    }

    Ok(json!({
        "id": message_id,
        "type": "message",
        "role": "assistant",
        "model": model,
        "content": build_content(text_parts.concat(), tool_calls),
        "stop_reason": stop_reason,
        "stop_sequence": Value::Null,
        "usage": usage,
    }))
}

fn remember_tool_call(
    tool_calls: &mut HashMap<String, ToolCallAccumulator>,
    tool_aliases: &mut HashMap<String, String>,
    item: &Value,
) {
    let item_id = item.get("id").and_then(Value::as_str).unwrap_or_default();
    let call_id = item
        .get("call_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(item_id)
        .to_string();
    let name = item
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if !item_id.is_empty() {
        tool_aliases.insert(item_id.to_string(), call_id.clone());
    }
    tool_calls
        .entry(call_id)
        .or_insert_with(|| ToolCallAccumulator {
            name,
            arguments: String::new(),
        });
}

fn finish_tool_call(
    tool_calls: &mut HashMap<String, ToolCallAccumulator>,
    tool_aliases: &mut HashMap<String, String>,
    item: &Value,
) {
    let item_id = item.get("id").and_then(Value::as_str).unwrap_or_default();
    let call_id = item
        .get("call_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(item_id)
        .to_string();
    if !item_id.is_empty() {
        tool_aliases.insert(item_id.to_string(), call_id.clone());
    }
    if item_id != call_id {
        if let Some(item_accumulator) = tool_calls.remove(item_id) {
            let call_accumulator = tool_calls.entry(call_id.clone()).or_default();
            if call_accumulator.name.is_empty() {
                call_accumulator.name = item_accumulator.name;
            }
            if call_accumulator.arguments.is_empty() {
                call_accumulator.arguments = item_accumulator.arguments;
            } else {
                call_accumulator
                    .arguments
                    .push_str(&item_accumulator.arguments);
            }
        }
    }
    let entry = tool_calls.entry(call_id).or_default();
    if let Some(name) = item.get("name").and_then(Value::as_str) {
        entry.name = name.to_string();
    }
    if let Some(arguments) = item.get("arguments").and_then(Value::as_str) {
        entry.arguments = arguments.to_string();
    }
}

fn canonical_tool_key(tool_aliases: &HashMap<String, String>, id: &str) -> String {
    tool_aliases
        .get(id)
        .cloned()
        .unwrap_or_else(|| id.to_string())
}

fn build_content(text: String, tool_calls: HashMap<String, ToolCallAccumulator>) -> Vec<Value> {
    let mut content = Vec::new();
    if !text.is_empty() {
        content.push(json!({ "type": "text", "text": text }));
    }
    for (call_id, tool_call) in tool_calls {
        let input =
            serde_json::from_str::<Value>(&tool_call.arguments).unwrap_or_else(|_| json!({}));
        content.push(json!({
            "type": "tool_use",
            "id": call_id,
            "name": tool_call.name,
            "input": input,
        }));
    }
    content
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

fn usage_from_responses(usage: Option<&Value>) -> Value {
    let Some(usage) = usage else {
        return json!({ "input_tokens": 0, "output_tokens": 0 });
    };
    json!({
        "input_tokens": usage.get("input_tokens").and_then(Value::as_u64).unwrap_or(0),
        "output_tokens": usage.get("output_tokens").and_then(Value::as_u64).unwrap_or(0),
    })
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

#[derive(Default)]
struct ToolCallAccumulator {
    name: String,
    arguments: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregates_responses_sse_to_anthropic_message() {
        let input = b"event: response.created\ndata: {\"response\":{\"id\":\"r1\",\"model\":\"gpt\"}}\n\n\
event: response.output_text.delta\ndata: {\"delta\":\"hi\"}\n\n\
event: response.completed\ndata: {\"response\":{\"status\":\"completed\",\"usage\":{\"input_tokens\":1,\"output_tokens\":2}}}\n\n";

        let output = responses_sse_to_anthropic_message(input).unwrap();

        assert_eq!(output["id"], "r1");
        assert_eq!(output["content"][0]["text"], "hi");
        assert_eq!(output["usage"]["input_tokens"], 1);
        assert_eq!(output["usage"]["output_tokens"], 2);
    }

    #[test]
    fn merges_tool_argument_deltas_by_item_id_into_call_id() {
        let input = b"event: response.output_item.added\ndata: {\"item\":{\"type\":\"function_call\",\"id\":\"item_1\",\"call_id\":\"call_1\",\"name\":\"lookup\"}}\n\n\
event: response.function_call_arguments.delta\ndata: {\"item_id\":\"item_1\",\"delta\":\"{\\\"q\\\":\"}\n\n\
event: response.function_call_arguments.delta\ndata: {\"item_id\":\"item_1\",\"delta\":\"\\\"hi\\\"}\"}\n\n\
event: response.output_item.done\ndata: {\"item\":{\"type\":\"function_call\",\"id\":\"item_1\",\"call_id\":\"call_1\",\"name\":\"lookup\"}}\n\n\
event: response.completed\ndata: {\"response\":{\"status\":\"completed\"}}\n\n";

        let output = responses_sse_to_anthropic_message(input).unwrap();
        let tool = output["content"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["type"] == "tool_use")
            .unwrap();

        assert_eq!(tool["id"], "call_1");
        assert_eq!(tool["name"], "lookup");
        assert_eq!(tool["input"]["q"], "hi");
        assert_eq!(output["content"].as_array().unwrap().len(), 1);
    }
}
