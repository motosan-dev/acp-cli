use std::io::{self, Write};

use super::OutputRenderer;

pub struct QuietRenderer;

impl QuietRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for QuietRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputRenderer for QuietRenderer {
    fn text_chunk(&mut self, text: &str) {
        print!("{text}");
        let _ = io::stdout().flush();
    }

    fn tool_status(&mut self, _tool: &str) {}
    fn tool_result(&mut self, _tool: &str, _output: &str) {}

    fn permission_denied(&mut self, _tool: &str) {}

    fn error(&mut self, _err: &str) {}

    fn session_info(&mut self, _id: &str) {}

    fn done(&mut self) {
        let _ = io::stdout().flush();
    }
}
