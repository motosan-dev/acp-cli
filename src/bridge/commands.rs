use super::events::PromptResult;
use crate::error::Result;
use tokio::sync::oneshot;

pub enum BridgeCommand {
    Prompt {
        messages: Vec<String>,
        reply: oneshot::Sender<Result<PromptResult>>,
    },
    Cancel,
    Shutdown,
    SetMode {
        mode: String,
        reply: oneshot::Sender<Result<()>>,
    },
    SetConfig {
        key: String,
        value: String,
        reply: oneshot::Sender<Result<()>>,
    },
}
