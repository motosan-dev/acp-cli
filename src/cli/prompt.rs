use crate::error::Result;

/// Run an interactive or piped prompt session (not yet implemented).
pub async fn run_prompt(
    _agent: &str,
    _prompt: &[String],
    _cwd: &str,
    _session: Option<&str>,
    _format: &str,
    _timeout: Option<u64>,
    _verbose: bool,
) -> Result<i32> {
    eprintln!("prompt not yet implemented");
    Ok(0)
}

/// Run a non-interactive exec command (not yet implemented).
pub async fn run_exec(
    _agent: &str,
    _prompt: &[String],
    _cwd: &str,
    _session: Option<&str>,
    _format: &str,
    _timeout: Option<u64>,
    _verbose: bool,
) -> Result<i32> {
    eprintln!("exec not yet implemented");
    Ok(0)
}
