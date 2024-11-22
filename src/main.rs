use crate::ui::PremiumUI;
use clap::{Arg, Command};
use console::style;

mod ui;
mod models;
mod commands;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ui = PremiumUI::new();
    ui.display_welcome()?;

    let cli = Command::new("omni")
        .about(format!("{}", style("Modern Development Environment CLI").cyan().bold()))
        .subcommand(
            Command::new("up")
                .about(format!("{}", style("Deploy application components").green()))
                .arg(
                    Arg::new("environment")
                        .long("env")
                        .help(&format!("Target environment {}", style("[dev/staging/prod]").yellow()))
                        .required(false)
                )
        )
        .subcommand(
            Command::new("push")
                .about(format!("{}", style("Push images to container registry").green()))
                .arg(
                    Arg::new("tag")
                        .long("tag")
                        .help(&format!("Image tag {}", style("[latest]").yellow()))
                        .required(false)
                )
        )
        .subcommand(
            Command::new("scale")
                .about(format!("{}", style("Scale application components").green()))
                .arg(
                    Arg::new("component")
                        .long("component")
                        .help(&format!("Component to scale {}", style("[frontend/backend/database]").yellow()))
                        .required(false)
                )
                .arg(
                    Arg::new("replicas")
                        .long("replicas")
                        .help(&format!("Number of replicas {}", style("[1-10]").yellow()))
                        .required(false)
                )
        )
        .get_matches();

    match cli.subcommand() {
        Some(("up", _)) => ui.deploy_interactive().await?,
        Some(("push", _)) => ui.push_interactive().await?,
        Some(("scale", _)) => ui.scale_interactive().await?,
        _ => {
            println!("\n{}", style("AVAILABLE COMMANDS:").magenta().bold());
            println!("  {} {}", style("up").cyan(), style("Deploy your application").dim());
            println!("  {} {}", style("push").cyan(), style("Push images to registry").dim());
            println!("  {} {}", style("scale").cyan(), style("Scale application components").dim());
            println!("\n{}", style("Use --help with any command for more information.").yellow());
        }
    }

    Ok(())
}