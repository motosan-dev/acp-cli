use serde::{Deserialize, Serialize};

/// Message from client to queue owner.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueueRequest {
    Prompt {
        messages: Vec<String>,
        reply_id: String,
    },
    Cancel,
    Status,
    SetMode {
        mode: String,
    },
    SetConfig {
        key: String,
        value: String,
    },
}

/// Message from queue owner to client.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueueResponse {
    PromptResult {
        reply_id: String,
        content: String,
        stop_reason: String,
    },
    Event {
        kind: String,
        data: String,
    },
    StatusResponse {
        state: String,
        queue_depth: usize,
    },
    Error {
        message: String,
    },
    Queued {
        reply_id: String,
        position: usize,
    },
    Ok,
}
