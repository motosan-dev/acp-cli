use clap::Parser;

use acp_cli::cli::{Cli, Commands, ConfigAction, SessionAction};
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
    let config = AcpCliConfig::load();

    // Resolve working directory
    let cwd = cli.cwd.clone().unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap()
            .to_string_lossy()
            .to_string()
    });

    // Resolve agent name: explicit flag > positional > config default > "claude"
    let agent = cli
        .agent_override
        .as_deref()
        .or(cli.agent.as_deref())
        .or(config.default_agent.as_deref())
        .unwrap_or("claude")
        .to_string();

    match cli.command {
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
        },
        Some(Commands::Exec { prompt }) => {
            acp_cli::cli::prompt::run_exec(
                &agent,
                &prompt,
                &cwd,
                cli.session.as_deref(),
                &cli.format,
                cli.timeout,
                cli.verbose,
            )
            .await
        }
        None => {
            // Implicit prompt mode: if prompt words given, run prompt; otherwise show help
            if !cli.prompt.is_empty() {
                acp_cli::cli::prompt::run_prompt(
                    &agent,
                    &cli.prompt,
                    &cwd,
                    cli.session.as_deref(),
                    &cli.format,
                    cli.timeout,
                    cli.verbose,
                )
                .await
            } else if cli.agent.is_some() {
                // Agent specified but no prompt text and no subcommand
                // Treat as interactive mode (stub for now)
                acp_cli::cli::prompt::run_prompt(
                    &agent,
                    &[],
                    &cwd,
                    cli.session.as_deref(),
                    &cli.format,
                    cli.timeout,
                    cli.verbose,
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
