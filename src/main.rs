use std::io::IsTerminal;
use std::path::PathBuf;

use clap::Parser;

use acp_cli::agent::registry::{default_registry, resolve_agent};
use acp_cli::cli::prompt_source::resolve_prompt;
use acp_cli::cli::{Cli, Commands, ConfigAction, SessionAction};
use acp_cli::client::permissions::PermissionMode;
use acp_cli::config::AcpCliConfig;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let exit_code = match run(cli).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            e.exit_code()
        }
    };
    std::process::exit(exit_code);
}

async fn run(cli: Cli) -> acp_cli::error::Result<i32> {
    // Resolve working directory
    let cwd = cli.cwd.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string()
    });

    // Load global config, then merge project-level overrides
    let global = AcpCliConfig::load();
    let project = AcpCliConfig::load_project(std::path::Path::new(&cwd));
    let config = global.merge(project);

    // Resolve agent name: explicit flag > positional > config default > "claude"
    let agent = cli
        .agent_override
        .as_deref()
        .or(cli.agent.as_deref())
        .or(config.default_agent.as_deref())
        .unwrap_or("claude")
        .to_string();

    match cli.command {
        Some(Commands::Init) => {
            acp_cli::cli::init::run_init()?;
            Ok(0)
        }
        Some(Commands::Config { action }) => match action {
            ConfigAction::Show => {
                let config_json = serde_json::to_string_pretty(&config).unwrap_or_else(|_| {
                    // Fallback: reload raw JSON from disk
                    let path = dirs::home_dir()
                        .map(|h| h.join(".acp-cli").join("config.json"))
                        .unwrap_or_default();
                    std::fs::read_to_string(path).unwrap_or_else(|_| "{}".to_string())
                });
                println!("{config_json}");
                Ok(0)
            }
        },
        Some(Commands::Sessions { action }) => match action {
            SessionAction::New { name } => {
                acp_cli::cli::session::sessions_new(&agent, &cwd, name.as_deref())?;
                Ok(0)
            }
            SessionAction::List => {
                acp_cli::cli::session::sessions_list(Some(&agent), Some(&cwd))?;
                Ok(0)
            }
            SessionAction::Close { name } => {
                acp_cli::cli::session::sessions_close(&agent, &cwd, name.as_deref())?;
                Ok(0)
            }
            SessionAction::Show { name } => {
                acp_cli::cli::session::sessions_show(&agent, &cwd, name.as_deref())?;
                Ok(0)
            }
            SessionAction::History { name } => {
                acp_cli::cli::session::sessions_history(&agent, &cwd, name.as_deref())?;
                Ok(0)
            }
        },
        Some(Commands::Cancel) => {
            let session_name = cli.session.as_deref();
            acp_cli::cli::session::cancel_prompt(&agent, &cwd, session_name)?;
            Ok(0)
        }
        Some(Commands::Status) => {
            let session_name = cli.session.as_deref();
            acp_cli::cli::session::session_status(&agent, &cwd, session_name)?;
            Ok(0)
        }
        Some(Commands::SetMode { ref mode }) => {
            let session_name = cli.session.as_deref();
            acp_cli::cli::session::set_mode(&agent, &cwd, session_name, mode).await?;
            Ok(0)
        }
        Some(Commands::Set { ref key, ref value }) => {
            let session_name = cli.session.as_deref();
            acp_cli::cli::session::set_config(&agent, &cwd, session_name, key, value).await?;
            Ok(0)
        }
        Some(Commands::Exec { ref prompt }) => {
            let stdin_is_tty = std::io::stdin().is_terminal();
            let prompt_text = resolve_prompt(cli.file.as_deref(), prompt, stdin_is_tty)?;
            if prompt_text.is_empty() {
                return Err(acp_cli::error::AcpCliError::Usage(
                    "exec requires a prompt".into(),
                ));
            }

            let (command, args) = resolve_agent_command(&agent, &config);
            let permission_mode = resolve_permission_mode(&cli, &config);

            acp_cli::cli::prompt::run_exec(
                command,
                args,
                PathBuf::from(&cwd),
                prompt_text,
                permission_mode,
                &cli.format,
                cli.timeout.or(config.timeout),
                cli.prompt_retries,
            )
            .await
        }
        None => {
            let stdin_is_tty = std::io::stdin().is_terminal();
            let has_file_flag = cli.file.is_some();
            let has_piped_stdin = !stdin_is_tty && cli.prompt.is_empty() && !has_file_flag;

            // Implicit prompt mode: file flag, piped stdin, positional args, or agent specified
            if !cli.prompt.is_empty() || cli.agent.is_some() || has_file_flag || has_piped_stdin {
                let prompt_text = resolve_prompt(cli.file.as_deref(), &cli.prompt, stdin_is_tty)?;
                if prompt_text.is_empty() {
                    // Agent specified but no prompt text -- show help
                    use clap::CommandFactory;
                    Cli::command().print_help().ok();
                    println!();
                    return Ok(0);
                }

                let (command, args) = resolve_agent_command(&agent, &config);
                let permission_mode = resolve_permission_mode(&cli, &config);

                acp_cli::cli::prompt::run_prompt(
                    &agent,
                    command,
                    args,
                    PathBuf::from(&cwd),
                    prompt_text,
                    cli.session.clone(),
                    permission_mode,
                    &cli.format,
                    cli.timeout.or(config.timeout),
                    cli.no_wait,
                    cli.prompt_retries,
                )
                .await
            } else {
                // No agent, no command -> print help
                use clap::CommandFactory;
                Cli::command().print_help().ok();
                println!();
                Ok(0)
            }
        }
    }
}

/// Resolve agent name to (command, args) using registry + config overrides.
fn resolve_agent_command(agent: &str, config: &AcpCliConfig) -> (String, Vec<String>) {
    let registry = default_registry();
    let overrides = config.agents.as_ref().cloned().unwrap_or_default();
    resolve_agent(agent, &registry, &overrides)
}

/// Determine permission mode from CLI flags, falling back to config default.
fn resolve_permission_mode(cli: &Cli, config: &AcpCliConfig) -> PermissionMode {
    if cli.approve_all {
        PermissionMode::ApproveAll
    } else if cli.deny_all {
        PermissionMode::DenyAll
    } else if cli.approve_reads {
        PermissionMode::ApproveReads
    } else {
        config
            .default_permissions
            .clone()
            .unwrap_or(PermissionMode::ApproveReads)
    }
}
