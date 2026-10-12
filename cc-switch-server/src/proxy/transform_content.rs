//! Content helpers for Messages -> Chat/Responses. Preserve prompt text and
//! unknown tool-result content; never reinterpret reminders as tool output.
//! Based on cc-switch's transform.rs / transform_responses.rs at 28dc5198.

use serde_json::{json, Value};

/// [1m] is Claude Code's context-window selector, not a model ID in Chat or
/// Responses. Native routes retain it because compatible gateways may use it.
pub fn converted_model(model: &str) -> &str {
    model
        .strip_suffix("[1m]")
        .or_else(|| model.strip_suffix("[1M]"))
        .unwrap_or(model)
}

pub fn system_to_text(system: &Value) -> String {
    if let Some(text) = system.as_str() {
        return strip_attribution(text).to_string();
    }
    system
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|block| block.get("text").and_then(Value::as_str))
                .map(strip_attribution)
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n")
        })
        .unwrap_or_default()
}

// Only the leading attribution line is transport metadata. Keep its following
// text, and keep occurrences inside ordinary prompt text. Native forwarding
// does not call this helper and preserves the entire system array.
fn strip_attribution(text: &str) -> &str {
    if !text.starts_with("x-anthropic-billing-header:") {
        return text;
    }
    let Some(end) = text.find(['\n', '\r']) else {
        return "";
    };
    let rest = &text[end..];
    let rest = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix(['\n', '\r']))
        .unwrap_or(rest);
    rest.strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix(['\n', '\r']))
        .unwrap_or(rest)
}

pub fn message_role(message: &Value) -> Result<&str, String> {
    let role = message
        .get("role")
        .and_then(Value::as_str)
        .unwrap_or("user");
    match role {
        "user" | "assistant" | "system" | "developer" => Ok(role),
        _ => Err(format!("Unsupported Messages role: {role}")),
    }
}

pub fn image_url(block: &Value) -> Option<String> {
    let source = block.get("source")?;
    match source.get("type").and_then(Value::as_str) {
        Some("url") => {
            let url = source.get("url").and_then(Value::as_str)?;
            return has_http_scheme(url).then(|| url.to_string());
        }
        Some("base64") | None => {}
        _ => return None,
    }
    let data = source
        .get("data")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())?;
    let media_type = source
        .get("media_type")
        .and_then(Value::as_str)
        .unwrap_or("image/png");
    Some(format!("data:{media_type};base64,{data}"))
}

fn has_http_scheme(url: &str) -> bool {
    url.get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
        || url
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
}

pub fn responses_part(block: &Value) -> Value {
    match block.get("type").and_then(Value::as_str) {
        Some("text") => {
            json!({"type":"input_text", "text":block.get("text").and_then(Value::as_str).unwrap_or("")})
        }
        Some("image") => {
            if let Some(url) = image_url(block) {
                json!({"type":"input_image", "image_url":url})
            } else {
                json_text(block)
            }
        }
        Some("document") => {
            if let Some(source) = block.get("source") {
                let filename = block
                    .get("title")
                    .or_else(|| block.get("filename"))
                    .and_then(Value::as_str)
                    .unwrap_or("document.pdf");
                match source.get("type").and_then(Value::as_str) {
                    Some("base64") => {
                        if let Some(data) = source
                            .get("data")
                            .and_then(Value::as_str)
                            .filter(|data| !data.is_empty())
                        {
                            let media_type = source
                                .get("media_type")
                                .and_then(Value::as_str)
                                .unwrap_or("application/pdf");
                            return json!({"type":"input_file", "filename":filename, "file_data":format!("data:{media_type};base64,{data}")});
                        }
                    }
                    Some("url") => {
                        if let Some(url) = source
                            .get("url")
                            .and_then(Value::as_str)
                            .filter(|url| has_http_scheme(url))
                        {
                            return json!({"type":"input_file", "filename":filename,"file_url":url});
                        }
                    }
                    _ => {}
                }
            }
            json_text(block)
        }
        _ => json_text(block),
    }
}

fn json_text(value: &Value) -> Value {
    json!({"type":"input_text", "text":value.to_string()})
}

pub fn tool_result_output(block: &Value) -> Value {
    let content = block.get("content").unwrap_or(&Value::Null);
    let is_error = block.get("is_error").and_then(Value::as_bool) == Some(true);
    if !is_error {
        if let Some(text) = content.as_str() {
            return json!(text);
        }
    }
    let mut parts = Vec::new();
    if is_error {
        parts.push(json!({"type":"input_text", "text":"[cc-switch:tool-result-error]"}));
    }
    match content {
        Value::Array(blocks) => parts.extend(blocks.iter().map(responses_part)),
        Value::String(text) => parts.push(json!({"type":"input_text", "text":text})),
        Value::Null => {}
        other => parts.push(json_text(other)),
    }
    json!(parts)
}

pub fn function_tools(body: &Value, chat: bool) -> Result<Vec<Value>, String> {
    let mut output = Vec::new();
    for tool in body
        .get("tools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if tool.get("type").and_then(Value::as_str) == Some("BatchTool") {
            continue;
        }
        if let Some(kind) = tool.get("type").and_then(Value::as_str) {
            if kind != "custom" {
                return Err(format!(
                    "Hosted tool type {kind} cannot be converted to a function tool"
                ));
            }
        }
        let name = tool
            .get("name")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or("Function tool is missing its name")?;
        let mut function = json!({"name":name, "parameters":tool.get("input_schema").cloned().unwrap_or_else(|| json!({"type":"object", "properties":{}}))});
        if let Some(description) = tool.get("description").and_then(Value::as_str) {
            function["description"] = json!(description);
        }
        // Native deferred loading has no equivalent here: load definitions
        // upfront, and preserve tool_reference results as JSON text.
        if chat {
            output.push(json!({"type":"function", "function":function}));
        } else {
            function["type"] = json!("function");
            function["strict"] = json!(false);
            output.push(function);
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strips_only_the_client_context_selector() {
        assert_eq!(converted_model("deepseek-v4-pro[1m]"), "deepseek-v4-pro");
        assert_eq!(converted_model("gpt-5[1M]"), "gpt-5");
        assert_eq!(converted_model("vendor-model-1m"), "vendor-model-1m");
    }
    #[test]
    fn strips_only_leading_attribution_and_preserves_real_prompt() {
        assert_eq!(
            system_to_text(
                &json!([{"text":"x-anthropic-billing-header: version=1\r\n\r\nKeep my rules"},{"text":"Literal:\nx-anthropic-billing-header: example"}])
            ),
            "Keep my rules\n\nLiteral:\nx-anthropic-billing-header: example"
        );
    }
    #[test]
    fn preserves_reference_image_and_error_in_tool_results() {
        let output = tool_result_output(
            &json!({"is_error":true,"content":[{"type":"tool_reference","tool_name":"mcp__test__read"},{"type":"image","source":{"type":"url","url":"https://example.com/image.png"}}]}),
        );
        assert!(output[0]["text"].as_str().unwrap().contains("error"));
        assert!(output[1]["text"]
            .as_str()
            .unwrap()
            .contains("mcp__test__read"));
        assert_eq!(output[2]["type"], "input_image");
    }

    #[test]
    fn media_sources_match_upstream_url_and_pdf_rules() {
        assert_eq!(
            image_url(&json!({"source":{"type":"url","url":"HTTPS://example.com/a.png"}})),
            Some("HTTPS://example.com/a.png".into())
        );
        assert!(image_url(&json!({"source":{"type":"file","data":"invalid"}})).is_none());
        assert_eq!(
            responses_part(
                &json!({"type":"document","filename":"report.pdf","source":{"type":"base64","data":"fixture"}})
            )["file_data"],
            "data:application/pdf;base64,fixture"
        );
        assert_eq!(
            responses_part(
                &json!({"type":"document","title":"linked.pdf","source":{"type":"url","url":"https://example.com/report.pdf"}})
            )["filename"],
            "linked.pdf"
        );
    }
}
