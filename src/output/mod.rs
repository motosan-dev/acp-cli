pub mod json;
pub mod quiet;
pub mod text;

pub trait OutputRenderer {
    fn text_chunk(&mut self, text: &str);
    fn tool_status(&mut self, tool: &str);
    fn permission_denied(&mut self, tool: &str);
    fn error(&mut self, err: &str);
    fn session_info(&mut self, id: &str);
    fn done(&mut self);
}
