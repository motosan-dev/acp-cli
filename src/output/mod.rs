pub mod json;
pub mod quiet;
pub mod text;

pub trait OutputRenderer {
    fn text_chunk(&mut self, text: &str);
    fn tool_status(&mut self, tool: &str);
    fn tool_result(&mut self, tool: &str, output: &str);
    fn permission_denied(&mut self, tool: &str);
    fn error(&mut self, err: &str);
    fn session_info(&mut self, id: &str);
    fn done(&mut self);
}

/// Returns `true` for tool names that perform file reads.
///
/// Used to decide whether `--suppress-reads` should hide the tool output.
/// The list is intentionally exhaustive and case-sensitive. When adding
/// support for a new agent whose read tool has a different name, add the
/// exact name string here — do not use case-insensitive matching, as that
/// would risk misidentifying non-read tools with similar names.
pub fn is_read_tool(name: &str) -> bool {
    matches!(
        name,
        "Read" | "read_file" | "readFile" | "fs/read" | "read" | "view_file"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_tool_names_are_detected() {
        assert!(is_read_tool("Read"));
        assert!(is_read_tool("read_file"));
        assert!(is_read_tool("readFile"));
        assert!(is_read_tool("fs/read"));
        assert!(is_read_tool("read"));
        assert!(is_read_tool("view_file"));
    }

    #[test]
    fn write_and_exec_tools_are_not_read_tools() {
        assert!(!is_read_tool("Write"));
        assert!(!is_read_tool("Bash"));
        assert!(!is_read_tool("Edit"));
        assert!(!is_read_tool("execute_command"));
        assert!(!is_read_tool("search_files"));
        assert!(!is_read_tool(""));
    }
}
