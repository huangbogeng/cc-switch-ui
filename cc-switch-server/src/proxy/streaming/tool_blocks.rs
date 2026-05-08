use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub(super) struct ToolBlockState {
    pub anthropic_index: u32,
    pub id: String,
    pub name: String,
    pub started: bool,
    pub pending_args: String,
    pub all_arguments: String,
}

pub(super) fn close_open_tool_blocks(
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

pub(super) fn guarded_openai_finish_reason(
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

fn openai_finish_reason(reason: Option<&str>) -> &'static str {
    match reason {
        Some("length") => "max_tokens",
        Some("tool_calls") | Some("function_call") => "tool_use",
        _ => "end_turn",
    }
}

fn is_valid_tool_block(state: &ToolBlockState) -> bool {
    if !state.started || state.id.is_empty() || state.name.is_empty() {
        return false;
    }
    let args = state.all_arguments.trim();
    args.is_empty() || serde_json::from_str::<Value>(args).is_ok()
}
