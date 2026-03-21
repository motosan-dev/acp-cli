use std::io::Read;
use std::path::Path;

use crate::error::{AcpCliError, Result};

/// Resolve the prompt text from the available sources.
///
/// Priority / conflict rules:
/// 1. `--file` and positional prompt args are mutually exclusive (Usage error).
/// 2. `--file -` reads all of stdin.
/// 3. `--file <path>` reads the file at that path.
/// 4. If no `--file` but stdin is piped (not a TTY), read stdin as the prompt.
/// 5. Otherwise, join the positional prompt words.
pub fn resolve_prompt(
    file_flag: Option<&str>,
    positional: &[String],
    stdin_is_terminal: bool,
) -> Result<String> {
    match file_flag {
        Some(path) => {
            // --file provided: positional prompt args must be empty
            if !positional.is_empty() {
                return Err(AcpCliError::Usage(
                    "cannot combine --file with positional prompt arguments".into(),
                ));
            }
            if path == "-" {
                read_stdin()
            } else {
                read_file(path)
            }
        }
        None => {
            if !stdin_is_terminal && positional.is_empty() {
                // stdin is piped and no positional args
                read_stdin()
            } else {
                Ok(positional.join(" "))
            }
        }
    }
}

fn read_stdin() -> Result<String> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    Ok(input.trim_end().to_string())
}

fn read_file(path: &str) -> Result<String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(AcpCliError::Usage(format!("file not found: {path}")));
    }
    let content = std::fs::read_to_string(p)?;
    Ok(content.trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positional_prompt_joined() {
        let result = resolve_prompt(None, &["hello".into(), "world".into()], true).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn file_flag_and_positional_is_error() {
        let result = resolve_prompt(Some("prompt.txt"), &["extra".into()], true);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("cannot combine"));
    }

    #[test]
    fn file_flag_reads_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("prompt.txt");
        std::fs::write(&file_path, "prompt from file\n").unwrap();

        let result = resolve_prompt(Some(file_path.to_str().unwrap()), &[], true).unwrap();
        assert_eq!(result, "prompt from file");
    }

    #[test]
    fn file_flag_missing_file_is_error() {
        let result = resolve_prompt(Some("/tmp/nonexistent-acp-cli-test-file.txt"), &[], true);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn no_file_no_positional_terminal_gives_empty() {
        let result = resolve_prompt(None, &[], true).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn empty_positional_with_file_dash_would_read_stdin() {
        // We can't easily test actual stdin reading in unit tests,
        // but we can verify the function dispatches correctly by checking
        // that --file - with positional args is an error.
        let result = resolve_prompt(Some("-"), &["extra".into()], true);
        assert!(result.is_err());
    }
}
