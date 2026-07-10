use crate::cleanup::prompt_builder::{
    BUG_REPORT_TRANSFORM_PROMPT, COMMIT_MESSAGE_TRANSFORM_PROMPT, DEVELOPER_TASK_TRANSFORM_PROMPT,
    DOCUMENTATION_TRANSFORM_PROMPT, POLISH_TRANSFORM_PROMPT, PROMPT_ENGINEER_TRANSFORM_PROMPT,
};

pub fn prompt_for(id: &str) -> Option<&'static str> {
    match id {
        "polish" => Some(POLISH_TRANSFORM_PROMPT),
        "prompt_engineer" => Some(PROMPT_ENGINEER_TRANSFORM_PROMPT),
        "developer_task" => Some(DEVELOPER_TASK_TRANSFORM_PROMPT),
        "bug_report" => Some(BUG_REPORT_TRANSFORM_PROMPT),
        "commit_message" => Some(COMMIT_MESSAGE_TRANSFORM_PROMPT),
        "documentation" => Some(DOCUMENTATION_TRANSFORM_PROMPT),
        _ => None,
    }
}

pub fn label_for(id: &str) -> &'static str {
    match id {
        "prompt_engineer" => "Prompt Engineer",
        "polish" => "Polish",
        _ => "Transform",
    }
}
