use std::io::{self, Write};

use serde_json::json;

use super::OutputRenderer;

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

    fn tool_result(&mut self, tool: &str, output: &str, is_read: bool) {
        self.emit(build_tool_result_event(
            tool,
            output,
            self.suppress_reads,
            is_read,
        ));
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

/// Build the JSON value for a `tool_result` event. Extracted for testability.
fn build_tool_result_event(
    tool: &str,
    output: &str,
    suppress_reads: bool,
    is_read: bool,
) -> serde_json::Value {
    if suppress_reads && is_read {
        json!({
            "type": "tool_result",
            "name": tool,
            "output": "[suppressed]",
            "suppressed": true,
        })
    } else {
        json!({
            "type": "tool_result",
            "name": tool,
            "output": output,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_result_suppressed_when_suppress_reads_and_is_read() {
        let v = build_tool_result_event("Read File", "file contents", true, true);
        assert_eq!(v["type"], "tool_result");
        assert_eq!(v["name"], "Read File");
        assert_eq!(v["output"], "[suppressed]");
        assert_eq!(v["suppressed"], true);
    }

    #[test]
    fn tool_result_not_suppressed_when_is_read_false() {
        let v = build_tool_result_event("Bash", "output text", true, false);
        assert_eq!(v["output"], "output text");
        assert!(v.get("suppressed").is_none());
    }

    #[test]
    fn tool_result_not_suppressed_when_suppress_reads_false() {
        let v = build_tool_result_event("Read File", "file contents", false, true);
        assert_eq!(v["output"], "file contents");
        assert!(v.get("suppressed").is_none());
    }
}
