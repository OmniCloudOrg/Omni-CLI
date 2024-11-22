use clap::{Arg, Command};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use console::{style, Term};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use spinners::{Spinner, Spinners};
use std::{thread, time::Duration};
use tabled::{Table, Tabled};
use anyhow::Result;

const LOGO: &str = r#"
   ____                  _ ______                    
  / __ \____ ___  ____  (_) ____/___  _________ ____ 
 / / / / __ `__ \/ __ \/ / /_  / __ \/ ___/ __ `/ _ \
/ /_/ / / / / / / / / / / __/ / /_/ / /  / /_/ /  __/
\____/_/ /_/ /_/_/ /_/_/_/    \____/_/   \__, /\___/ 
                                        /____/       
"#;

#[derive(Debug, Tabled)]
struct ComponentStatus {
    #[tabled(rename = "Component")]
    name: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Replicas")]
    replicas: String,
    #[tabled(rename = "CPU")]
    cpu: String,
    #[tabled(rename = "Memory")]
    memory: String,
}

struct PremiumUI {
    term: Term,
    multi_progress: MultiProgress,
    theme: ColorfulTheme,
}

impl PremiumUI {
    fn new() -> Self {
        Self {
            term: Term::stdout(),
            multi_progress: MultiProgress::new(),
            theme: ColorfulTheme::default(),
        }
    }

    fn display_welcome(&self) -> Result<()> {
        self.term.clear_screen()?;
        println!("{}", style(LOGO).cyan().bold());
        println!("{}", style("Welcome to Omniforge - Modern Development Environment").cyan().bold());
        println!("{}\n", style("Version 1.0.0").dim());
        Ok(())
    }

    fn create_spinner(&self, message: &str) -> Spinner {
        Spinner::with_timer(Spinners::Dots12, message.into())
    }

    fn create_progress_bar(&self, len: u64, message: &str) -> ProgressBar {
        let pb = self.multi_progress.add(ProgressBar::new(len));
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("=>-"));
        pb.set_message(message.to_string());
        pb
    }

    async fn deploy_interactive(&self) -> Result<()> {
        // Project path selection
        let project_path: String = Input::with_theme(&self.theme)
            .with_prompt("Enter project path")
            .default(".".into())
            .interact_text()?;

        // Environment selection
        let environments = vec!["Development", "Staging", "Production"];
        let env_selection = Select::with_theme(&self.theme)
            .with_prompt("Select deployment environment")
            .items(&environments)
            .default(0)
            .interact()?;

        // Confirmation with warning for production
        if environments[env_selection] == "Production" {
            let confirm = Confirm::with_theme(&self.theme)
                .with_prompt("⚠️  You're deploying to production. Are you sure?")
                .default(false)
                .interact()?;

            if !confirm {
                println!("{}", style("Deployment cancelled.").yellow());
                return Ok(());
            }
        }

        // Initialize deployment
        println!("\n{}", style("🚀 Initializing deployment...").cyan().bold());
        
        // Simulated deployment steps with rich progress indicators
        let steps = [
            ("Analyzing project", 20),
            ("Building containers", 40),
            ("Pushing to registry", 30),
            ("Configuring services", 25),
            ("Starting components", 35)
        ];

        for (step, duration) in steps.iter() {
            let pb = self.create_progress_bar(*duration, step);
            for i in 0..*duration {
                pb.inc(1);
                thread::sleep(Duration::from_millis(100));
                
                // Add detailed progress messages
                match i {
                    5 => pb.set_message(format!("{} (scanning dependencies)", step)),
                    15 => pb.set_message(format!("{} (optimizing)", step)),
                    25 => pb.set_message(format!("{} (finalizing)", step)),
                    _ => {}
                }
            }
            pb.finish_with_message(format!("{} ✓", step));
        }

        // Display deployment summary
        let status_table = Table::new(vec![
            ComponentStatus {
                name: "Web Frontend".into(),
                status: "Running".into(),
                replicas: "3/3".into(),
                cpu: "150m".into(),
                memory: "256Mi".into(),
            },
            ComponentStatus {
                name: "API Backend".into(),
                status: "Running".into(),
                replicas: "2/2".into(),
                cpu: "200m".into(),
                memory: "512Mi".into(),
            },
            ComponentStatus {
                name: "Database".into(),
                status: "Running".into(),
                replicas: "1/1".into(),
                cpu: "500m".into(),
                memory: "1Gi".into(),
            },
        ]).to_string();

        println!("\n{}", style("📊 Deployment Status").cyan().bold());
        println!("{}", status_table);

        // Display endpoints
        println!("\n{}", style("🌍 Application Endpoints").cyan().bold());
        println!("Frontend: {}", style("https://app.example.com").green());
        println!("API:      {}", style("https://api.example.com").green());
        println!("Metrics:  {}", style("https://metrics.example.com").green());

        // Final success message
        println!("\n{}", style("✨ Deployment completed successfully!").green().bold());
        println!("{}", style("Run 'omni status' to monitor your deployment.").dim());

        Ok(())
    }

    async fn scale_interactive(&self) -> Result<()> {
        // Component selection
        let components = vec!["Web Frontend", "API Backend", "Database"];
        let component = Select::with_theme(&self.theme)
            .with_prompt("Select component to scale")
            .items(&components)
            .interact()?;

        // Replica count input
        let replicas: u32 = Input::with_theme(&self.theme)
            .with_prompt("Enter number of replicas")
            .validate_with(|input: &String| -> Result<(), &str> {
                match input.parse::<u32>() {
                    Ok(n) if n > 0 && n <= 10 => Ok(()),
                    _ => Err("Please enter a number between 1 and 10")
                }
            })
            .interact_text()?
            .parse()?;

        let mut spinner = self.create_spinner("Scaling component...");
        thread::sleep(Duration::from_secs(2));
        spinner.stop_with_message("✓ Scaling completed successfully!".to_string());

        // Display new status
        println!("\n{}", style("📊 Updated Component Status").cyan().bold());
        let status = Table::new(vec![ComponentStatus {
            name: components[component].into(),
            status: "Running".into(),
            replicas: format!("{}/{}", replicas, replicas),
            cpu: format!("{}m", replicas * 150),
            memory: format!("{}Mi", replicas * 256),
        }]).to_string();
        println!("{}", status);

        Ok(())
    }

    async fn push_interactive(&self) -> Result<()> {
        // Image tag input
        let tag: String = Input::with_theme(&self.theme)
            .with_prompt("Enter image tag")
            .default("latest".into())
            .interact_text()?;

        // Registry selection
        let registries = vec!["Docker Hub", "Google Container Registry", "Amazon ECR"];
        let registry = Select::with_theme(&self.theme)
            .with_prompt("Select registry")
            .items(&registries)
            .interact()?;

        println!("\n{}", style("📦 Pushing image...").cyan().bold());
        
        let pb = self.create_progress_bar(100, "Preparing image");
        for i in 0..100 {
            pb.inc(1);
            thread::sleep(Duration::from_millis(50));
            
            match i {
                20 => pb.set_message("Building layers..."),
                50 => pb.set_message("Optimizing image..."),
                80 => pb.set_message("Pushing to registry..."),
                _ => {}
            }
        }
        pb.finish_with_message("✓ Image pushed successfully!");

        // Display image details
        println!("\n{}", style("🏷️  Image Details").cyan().bold());
        println!("Registry: {}", style(registries[registry]).green());
        println!("Tag:      {}", style(tag).green());
        println!("Size:     {}", style("156.4 MB").green());
        println!("Layers:   {}", style("12").green());

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let ui = PremiumUI::new();
    ui.display_welcome()?;

    let cli = Command::new("omni")
        .about(format!("{}", style("Modern Development Environment CLI").cyan().bold()))
        // Commands section
        .subcommand(
            Command::new("up")
                .about(format!("{}", style("Deploy application components").green()))
                .arg(
                    Arg::new("environment")
                        .long("env")
                        .help(&format!("Target environment {}",
                            style("[dev/staging/prod]").yellow()))
                        .required(false)
                )
        )
        .subcommand(
            Command::new("push")
                .about(format!("{}", style("Push images to container registry").green()))
                .arg(
                    Arg::new("tag")
                        .long("tag")
                        .help(&format!("Image tag {}",
                            style("[latest]").yellow()))
                        .required(false)
                )
        )
        .subcommand(
            Command::new("scale")
                .about(format!("{}", style("Scale application components").green()))
                .arg(
                    Arg::new("component")
                        .long("component")
                        .help(&format!("Component to scale {}",
                            style("[frontend/backend/database]").yellow()))
                        .required(false)
                )
                .arg(
                    Arg::new("replicas")
                        .long("replicas")
                        .help(&format!("Number of replicas {}",
                            style("[1-10]").yellow()))
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