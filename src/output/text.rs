use std::io::{self, IsTerminal, Write};

use indicatif::{ProgressBar, ProgressStyle};

use super::{OutputRenderer, is_read_tool};

pub struct TextRenderer {
    spinner: Option<ProgressBar>,
    is_tty: bool,
    suppress_reads: bool,
}

impl TextRenderer {
    pub fn new(suppress_reads: bool) -> Self {
        let is_tty = io::stdout().is_terminal();
        Self {
            spinner: None,
            is_tty,
            suppress_reads,
        }
    }

    fn clear_spinner(&mut self) {
        if let Some(spinner) = self.spinner.take() {
            spinner.finish_and_clear();
        }
    }

    fn show_spinner(&mut self, message: &str) {
        self.clear_spinner();
        if self.is_tty {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::with_template("{spinner:.cyan} {msg}")
                    .unwrap()
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
            );
            pb.set_message(message.to_string());
            pb.enable_steady_tick(std::time::Duration::from_millis(80));
            self.spinner = Some(pb);
        } else {
            eprintln!("{message}");
        }
    }
}

impl OutputRenderer for TextRenderer {
    fn text_chunk(&mut self, text: &str) {
        self.clear_spinner();
        print!("{text}");
        let _ = io::stdout().flush();
    }

    fn tool_status(&mut self, tool: &str) {
        self.show_spinner(&format!("Using tool: {tool}"));
    }

    fn tool_result(&mut self, tool: &str, output: &str) {
        self.clear_spinner();
        if self.suppress_reads && is_read_tool(tool) {
            eprintln!("  [read suppressed — {} bytes]", output.len());
        }
        // Non-read tools: result body is not displayed in text mode to avoid
        // cluttering the terminal. The agent's own response text (via TextChunk)
        // describes what was found.
    }

    fn permission_denied(&mut self, tool: &str) {
        self.clear_spinner();
        eprintln!("Permission denied: {tool}");
    }

    fn error(&mut self, err: &str) {
        self.clear_spinner();
        eprintln!("Error: {err}");
    }

    fn session_info(&mut self, id: &str) {
        self.clear_spinner();
        eprintln!("Session: {id}");
    }

    fn done(&mut self) {
        self.clear_spinner();
        let _ = io::stdout().flush();
    }
}
