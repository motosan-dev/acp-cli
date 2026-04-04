pub mod json;
pub mod quiet;
pub mod text;

pub trait OutputRenderer {
    fn text_chunk(&mut self, text: &str);
    fn tool_status(&mut self, tool: &str);
    /// Called when a tool call completes. `is_read` is true for file-read tools.
    fn tool_result(&mut self, tool: &str, output: &str, is_read: bool);
    fn permission_denied(&mut self, tool: &str);
    fn error(&mut self, err: &str);
    fn session_info(&mut self, id: &str);
    fn done(&mut self);
}
