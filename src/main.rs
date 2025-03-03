// main.rs
use crate::ui::PremiumUI;
use clap::{Arg, Command};
use console::style;

mod commands;
mod models;
mod ui;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ui = PremiumUI::new();
    ui.display_welcome()?;

    let cli = Command::new("omni")
        .about(format!(
            "{}",
            style("OmniOrchestrator - Self-Hosted Cloud Platform CLI").cyan().bold()
        ))
        .subcommand(
            Command::new("init")
                .about(format!(
                    "{}",
                    style("Initialize cloud environment with OmniOrchestrator").green()
                ))
                .arg(
                    Arg::new("force")
                        .long("force")
                        .help("Force re-initialization even if config exists")
                        .required(false)
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("hosts")
                .about(format!(
                    "{}",
                    style("List configured SSH hosts").green()
                )),
        )
        .subcommand(
            Command::new("status")
                .about(format!(
                    "{}",
                    style("Check OmniOrchestrator status").green()
                )),
        )
        .subcommand(
            Command::new("up")
                .about(format!(
                    "{}",
                    style("Deploy application components").green()
                ))
                .arg(
                    Arg::new("environment")
                        .long("env")
                        .help(&format!(
                            "Target environment {}",
                            style("[dev/staging/prod]").yellow()
                        ))
                        .required(false),
                ),
        )
        .subcommand(
            Command::new("push")
                .about(format!(
                    "{}",
                    style("Push images to container registry").green()
                ))
                .arg(
                    Arg::new("tag")
                        .long("tag")
                        .help(&format!("Image tag {}", style("[latest]").yellow()))
                        .required(false),
                ),
        )
        .subcommand(
            Command::new("scale")
                .about(format!("{}", style("Scale application components").green()))
                .arg(
                    Arg::new("component")
                        .long("component")
                        .help(&format!(
                            "Component to scale {}",
                            style("[frontend/backend/database]").yellow()
                        ))
                        .required(false),
                )
                .arg(
                    Arg::new("replicas")
                        .long("replicas")
                        .help(&format!("Number of replicas {}", style("[1-10]").yellow()))
                        .required(false),
                ),
        )
        .subcommand(
            Command::new("logs")
                .about(format!("{}", style("View application logs").green()))
                .arg(
                    Arg::new("host")
                        .long("host")
                        .help("Host to view logs from")
                        .required(false),
                )
                .arg(
                    Arg::new("service")
                        .long("service")
                        .help("Service to view logs for")
                        .required(false),
                )
                .arg(
                    Arg::new("tail")
                        .long("tail")
                        .help("Number of lines to show")
                        .default_value("100"),
                ),
        )
        .subcommand(
            Command::new("service")
                .about(format!("{}", style("Manage OmniOrchestrator services").green()))
                .subcommand(
                    Command::new("restart")
                        .about("Restart a service")
                        .arg(Arg::new("host").required(true))
                        .arg(Arg::new("service").required(true)),
                )
                .subcommand(
                    Command::new("stop")
                        .about("Stop a service")
                        .arg(Arg::new("host").required(true))
                        .arg(Arg::new("service").required(true)),
                )
                .subcommand(
                    Command::new("start")
                        .about("Start a service")
                        .arg(Arg::new("host").required(true))
                        .arg(Arg::new("service").required(true)),
                ),
        )
        .subcommand(
            Command::new("backup")
                .about(format!("{}", style("Manage backup operations").green()))
                .subcommand(Command::new("now").about("Trigger an immediate backup"))
                .subcommand(Command::new("list").about("List available backups"))
                .subcommand(
                    Command::new("restore")
                        .about("Restore from a backup")
                        .arg(Arg::new("id").required(true)),
                ),
        )
        .subcommand(
            Command::new("rollback")
                .about(format!("{}", style("Rollback to previous version").green()))
                .arg(
                    Arg::new("version")
                        .long("version")
                        .help("Version to rollback to")
                        .required(false),
                ),
        )
        .subcommand(
            Command::new("config")
                .about(format!(
                    "{}",
                    style("Manage application configuration").green()
                ))
                .subcommand(Command::new("view").about("View current configuration"))
                .subcommand(Command::new("edit").about("Edit configuration"))
                .subcommand(Command::new("reset").about("Reset configuration to defaults")),
        )
        .get_matches();

    match cli.subcommand() {
        // OmniOrchestrator commands
        Some(("init", _)) => ui.init_environment().await?,
        Some(("hosts", _)) => ui.list_ssh_hosts().await?,
        Some(("status", _)) => ui.orchestrator_status().await?,
        
        // Application deployment commands
        Some(("up", _)) => ui.deploy_interactive().await?,
        Some(("push", _)) => ui.push_interactive().await?,
        Some(("scale", _)) => ui.scale_interactive().await?,
        Some(("logs", _)) => ui.logs_interactive().await?,
        Some(("rollback", _)) => ui.rollback_interactive().await?,
        
        // Service management
        Some(("service", subcommand)) => match subcommand.subcommand() {
            Some(("restart", _)) => println!("{}", style("Service restart not yet implemented").yellow()),
            Some(("stop", _)) => println!("{}", style("Service stop not yet implemented").yellow()),
            Some(("start", _)) => println!("{}", style("Service start not yet implemented").yellow()),
            _ => println!("{}", style("Use 'omni service --help' for available commands").yellow()),
        },
        
        // Backup management
        Some(("backup", subcommand)) => match subcommand.subcommand() {
            Some(("now", _)) => println!("{}", style("Backup now not yet implemented").yellow()),
            Some(("list", _)) => println!("{}", style("Backup list not yet implemented").yellow()),
            Some(("restore", _)) => println!("{}", style("Backup restore not yet implemented").yellow()),
            _ => println!("{}", style("Use 'omni backup --help' for available commands").yellow()),
        },
        
        // Configuration management
        Some(("config", subcommand)) => match subcommand.subcommand() {
            Some(("view", _)) => ui.config_view().await?,
            Some(("edit", _)) => ui.config_edit().await?,
            Some(("reset", _)) => ui.config_reset().await?,
            _ => ui.config_view().await?,
        },
        
        // Help menu
        _ => {
            println!("\n{}", style("OMNI ORCHESTRATOR COMMANDS:").magenta().bold());
            println!(
                "  {} {}",
                style("init").cyan(),
                style("Initialize cloud environment").dim()
            );
            println!(
                "  {} {}",
                style("hosts").cyan(),
                style("List configured SSH hosts").dim()
            );
            println!(
                "  {} {}",
                style("status").cyan(),
                style("Check OmniOrchestrator status").dim()
            );
            println!(
                "  {} {}",
                style("service").cyan(),
                style("Manage OmniOrchestrator services").dim()
            );
            println!(
                "  {} {}",
                style("backup").cyan(),
                style("Manage backup operations").dim()
            );
            
            println!("\n{}", style("APPLICATION COMMANDS:").magenta().bold());
            println!(
                "  {} {}",
                style("up").cyan(),
                style("Deploy your application").dim()
            );
            println!(
                "  {} {}",
                style("push").cyan(),
                style("Push images to registry").dim()
            );
            println!(
                "  {} {}",
                style("scale").cyan(),
                style("Scale application components").dim()
            );
            println!(
                "  {} {}",
                style("logs").cyan(),
                style("View application logs").dim()
            );
            println!(
                "  {} {}",
                style("rollback").cyan(),
                style("Rollback to previous version").dim()
            );
            println!(
                "  {} {}",
                style("config").cyan(),
                style("Manage application configuration").dim()
            );
            println!(
                "\n{}",
                style("Use --help with any command for more information.").yellow()
            );
        }
    }

    Ok(())
}