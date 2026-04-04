use std::io::{self, Write};

use serde_json::json;

use super::{OutputRenderer, is_read_tool};

pub struct JsonRenderer {
    suppress_reads: bool,
}

impl JsonRenderer {
    pub fn new(suppress_reads: bool) -> Self {
        Self { suppress_reads }
    }

    fn emit(&self, value: serde_json::Value) {
        let line = serde_json::to_string(&value).expect("failed to serialize JSON output");
        println!("{line}");
        let _ = io::stdout().flush();
    }
}

impl OutputRenderer for JsonRenderer {
    fn text_chunk(&mut self, text: &str) {
        self.emit(json!({"type": "text", "content": text}));
    }

    fn tool_status(&mut self, tool: &str) {
        self.emit(json!({"type": "tool", "name": tool}));
    }

    fn tool_result(&mut self, tool: &str, output: &str) {
        if self.suppress_reads && is_read_tool(tool) {
            self.emit(json!({
                "type": "tool_result",
                "name": tool,
                "output": "[suppressed]",
                "suppressed": true,
            }));
        } else {
            self.emit(json!({
                "type": "tool_result",
                "name": tool,
                "output": output,
            }));
        }
    }

    fn permission_denied(&mut self, tool: &str) {
        self.emit(json!({"type": "error", "message": format!("permission denied: {tool}")}));
    }

    fn error(&mut self, err: &str) {
        self.emit(json!({"type": "error", "message": err}));
    }

    fn session_info(&mut self, id: &str) {
        self.emit(json!({"type": "session", "sessionId": id}));
    }

    fn done(&mut self) {
        self.emit(json!({"type": "done"}));
    }
}
