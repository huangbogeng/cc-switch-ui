//! OpenAI Responses SSE aggregation for non-streaming Anthropic clients.

use super::reasoning_bridge::anthropic_block_from_openai_reasoning_item;
use super::responses_contract::{
    stop_reason as response_stop_reason, terminal_error, usage as usage_from_responses,
};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Public Responses endpoints can return JSON; Codex OAuth requires SSE even
/// for a non-streaming Messages client. Keep both contracts explicit.
pub fn responses_json_to_anthropic_message(response: &Value) -> Result<Value, String> {
    if let Some(error) = terminal_error(response) {
        return Err(error);
    }
    let output = response
        .get("output")
        .and_then(Value::as_array)
        .ok_or("Responses JSON is missing output")?;
    let mut content = Vec::new();
    for item in output {
        match item.get("type").and_then(Value::as_str) {
            Some("message") => {
                for part in item
                    .get("content")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    if let Some(text) = part
                        .get("text")
                        .or_else(|| part.get("refusal"))
                        .and_then(Value::as_str)
                    {
                        content.push(json!({"type":"text", "text":text}));
                    }
                }
            }
            Some("function_call") => {
                let arguments = item
                    .get("arguments")
                    .and_then(Value::as_str)
                    .unwrap_or("{}");
                content.push(json!({"type":"tool_use", "id":item.get("call_id"), "name":item.get("name"), "input":serde_json::from_str::<Value>(arguments).map_err(|e|e.to_string())?}));
            }
            Some("reasoning") => {
                if let Some(block) = anthropic_block_from_openai_reasoning_item(item) {
                    content.push(block);
                }
            }
            other => return Err(format!("Unsupported Responses output item: {other:?}")),
        }
    }
    let stop = response_stop_reason(response, content.iter().any(|c| c["type"] == "tool_use"));
    Ok(
        json!({"id":response.get("id"),"type":"message","role":"assistant","model":response.get("model"),"content":content,"stop_reason":stop,"stop_sequence":Value::Null,"usage":usage_from_responses(response.get("usage"))}),
    )
}

pub fn responses_sse_to_anthropic_message(bytes: &[u8]) -> Result<Value, String> {
    let text = String::from_utf8_lossy(bytes).replace("\r\n", "\n");
    let mut message_id = String::new();
    let mut model = String::new();
    let mut text_parts = Vec::new();
    let mut usage = json!({ "input_tokens": 0, "output_tokens": 0 });
    let mut stop_reason = "end_turn";
    let mut tool_calls: HashMap<String, ToolCallAccumulator> = HashMap::new();
    let mut tool_aliases: HashMap<String, String> = HashMap::new();
    let mut reasoning = Vec::new();
    let mut terminal = false;

    for block in text.split("\n\n") {
        let Some((event_name, data)) = parse_sse_block(block) else {
            continue;
        };
        let Ok(data) = serde_json::from_str::<Value>(&data) else {
            continue;
        };

        match event_name
            .as_deref()
            .or_else(|| data.get("type").and_then(Value::as_str))
        {
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
                    tool_calls.entry(key).or_default().arguments.push_str(delta);
                }
            }
            Some("response.output_item.done") => {
                if let Some(item) = data.get("item") {
                    if item.get("type").and_then(Value::as_str) == Some("function_call") {
                        finish_tool_call(&mut tool_calls, &mut tool_aliases, item);
                    } else if let Some(block) = anthropic_block_from_openai_reasoning_item(item) {
                        reasoning.push(block);
                    }
                }
            }
            Some("response.completed") | Some("response.incomplete") => {
                let response = data.get("response").unwrap_or(&data);
                if let Some(error) = terminal_error(response) {
                    return Err(error);
                }
                // Complete snapshots are authoritative and preserve output order.
                if response.get("output").and_then(Value::as_array).is_some() {
                    return responses_json_to_anthropic_message(response);
                }
                terminal = true;
                stop_reason = response_stop_reason(response, !tool_calls.is_empty());
                usage = usage_from_responses(response.get("usage"));
                if let Some(id) = response.get("id").and_then(Value::as_str) {
                    message_id = id.to_string();
                }
                if let Some(response_model) = response.get("model").and_then(Value::as_str) {
                    model = response_model.to_string();
                }
                break;
            }
            Some("error" | "response.failed" | "response.cancelled") => {
                return Err(data.to_string())
            }
            _ => {}
        }
    }

    if !terminal {
        return Err("Responses stream ended without a terminal event".into());
    }
    reasoning.extend(build_content(text_parts.concat(), tool_calls));
    Ok(json!({
        "id": message_id,
        "type": "message",
        "role": "assistant",
        "model": model,
        "content": reasoning,
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
    fn rejects_failed_cancelled_and_truncated_responses() {
        for status in ["failed", "cancelled"] {
            let response =
                json!({"status":status,"output":[],"error":{"message":"backend unavailable"}});
            assert!(responses_json_to_anthropic_message(&response)
                .unwrap_err()
                .contains("backend unavailable"));
            let sse = format!("event: response.{status}\ndata: {{\"response\":{response}}}\n\n");
            assert!(responses_sse_to_anthropic_message(sse.as_bytes()).is_err());
        }
        assert!(
            responses_sse_to_anthropic_message(b"event: response.created\ndata: {}\n\n").is_err()
        );
    }

    #[test]
    fn terminal_snapshot_preserves_order_and_ignores_late_events() {
        let response = json!({"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"先检查"}]},{"type":"reasoning","id":"rs","summary":[],"encrypted_content":"opaque"},{"type":"function_call","call_id":"call","name":"Read","arguments":"{}"}]});
        let sse=format!("event: response.completed\r\ndata: {{\"response\":{response}}}\r\n\r\nevent: response.failed\r\ndata: {{}}\r\n\r\n");
        let result = responses_sse_to_anthropic_message(sse.as_bytes()).unwrap();
        assert_eq!(result["content"][0]["text"], "先检查");
        assert_eq!(result["content"][1]["type"], "redacted_thinking");
        assert_eq!(result["content"][2]["type"], "tool_use");
        assert_eq!(result["stop_reason"], "tool_use");
    }

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
