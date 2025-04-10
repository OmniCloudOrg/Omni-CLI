use anyhow::{Context, Result};
use console::style;
use dialoguer::{Confirm, Input, MultiSelect, Select};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tabled::{Table, Tabled};
use tokio::time::Duration;

use crate::ui::PremiumUI;

#[derive(Debug, Deserialize)]
struct ApiResponse {
    status: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SshHost {
    name: String,
    hostname: String,
    username: String,
    password: Option<String>,
    port: u16,
    identity_file: Option<String>,
    is_bastion: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudConfig {
    company_name: String,
    admin_name: String,
    cloud_name: String,
    region: String,
    ssh_hosts: Vec<SshHost>,
    enable_monitoring: bool,
    enable_backups: bool,
    backup_retention_days: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HostDeploymentStatus {
    host: String,
    status: String,
    services: Vec<ServiceStatus>,
    current_step: String,
    progress: u8,
    error: Option<String>,
    completed: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceStatus {
    name: String,
    status: String,
    uptime: Option<String>,
    cpu: Option<String>,
    memory: Option<String>,
}

#[derive(Tabled)]
struct SshHostDisplay {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Hostname")]
    hostname: String,
    #[tabled(rename = "Username")]
    username: String,
    #[tabled(rename = "Password")]
    password: String,
    #[tabled(rename = "Port")]
    port: String,
    #[tabled(rename = "Identity File")]
    identity_file: String,
    #[tabled(rename = "Bastion")]
    is_bastion: String,
}

#[derive(Tabled)]
struct ServiceStatusDisplay {
    #[tabled(rename = "Host")]
    host: String,
    #[tabled(rename = "Service")]
    service: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Uptime")]
    uptime: String,
    #[tabled(rename = "CPU")]
    cpu: String,
    #[tabled(rename = "Memory")]
    memory: String,
}

impl From<&SshHost> for SshHostDisplay {
    fn from(host: &SshHost) -> Self {
        SshHostDisplay {
            name: host.name.clone(),
            hostname: host.hostname.clone(),
            username: host.username.clone(),
            password: "***".to_string(),
            port: host.port.to_string(),
            identity_file: host
                .identity_file
                .clone()
                .unwrap_or_else(|| "-".to_string()),
            is_bastion: if host.is_bastion { "Yes" } else { "No" }.to_string(),
        }
    }
}

impl PremiumUI {
    pub async fn init_environment(&self) -> Result<()> {
        let config_dir = "config";
        let config_path = format!("{}/cloud-config.json", config_dir);
        let config = if Path::new(&config_path).exists() {
            println!(
                "\n{}",
                style("📋 Using existing configuration").cyan().bold()
            );
            let config_json =
                fs::read_to_string(&config_path).context("Failed to read configuration file")?;
            let config: CloudConfig =
                serde_json::from_str(&config_json).context("Failed to parse configuration")?;

            // Display summary of loaded configuration
            println!("Company: {}", style(&config.company_name).green());
            println!("Cloud Name: {}", style(&config.cloud_name).green());
            println!("SSH Hosts: {}", style(config.ssh_hosts.len()).green());

            config
        } else {
            println!(
                "\n{}",
                style("🚀 Cloud Environment Configuration").cyan().bold()
            );
            println!(
                "{}",
                style("This wizard will help you configure your self-hosted cloud environment.")
                    .dim()
            );

            // Basic cloud platform configuration
            let company_name: String = Input::with_theme(&self.theme)
                .with_prompt("Company name")
                .interact_text()?;

            let admin_name: String = Input::with_theme(&self.theme)
                .with_prompt("Your name (admin)")
                .interact_text()?;

            let cloud_name: String = Input::with_theme(&self.theme)
                .with_prompt("Cloud platform name")
                .default(format!(
                    "{}-cloud",
                    company_name.to_lowercase().replace(" ", "-")
                ))
                .interact_text()?;

            // Fetch regions from API
            println!("{}", style("Fetching available regions...").dim());
            let regions_response = match self.api_client.get::<Vec<libomni::types::db::v1::Region>>("/regions").await {
                Ok(response) => {
                    response
                },
                Err(err) => {
                    println!("{}", style("Failed to fetch regions from API").red());
                    println!("{}", style(format!("Error: {:?}", err)).red());
                    return Err(anyhow::anyhow!("Failed to fetch regions from API: {}", err));
                }
            };

            if regions_response.is_empty() {
                println!("{}", style("No regions found. Using default region.").yellow());
            } else {
                println!(
                    "{}",
                    style(format!("Found {} regions", regions_response.len())).green()
                );
            }

            // Create list of region names from API response
            let mut regions: Vec<String> = regions_response
                .iter()
                .filter(|r| r.status == "active")
                .map(|r| r.name.clone())
                .collect();
            regions.push("custom".to_string());
            let region_selection = Select::with_theme(&self.theme)
                .with_prompt("Select primary region")
                .items(&regions)
                .default(0)
                .interact()?;

            let region = if regions[region_selection] == "custom" {
                Input::with_theme(&self.theme)
                    .with_prompt("Enter custom region")
                    .interact_text()?
            } else {
                regions[region_selection].to_string()
            };

            // SSH hosts configuration
            let mut ssh_hosts = Vec::new();
            println!("\n{}", style("📡 SSH Host Configuration").cyan().bold());
            println!(
                "{}",
                style("Configure SSH hosts for your cloud environment").dim()
            );

            loop {
                // Display current hosts if any exist
                if !ssh_hosts.is_empty() {
                    println!("\n{}", style("Current SSH Hosts:").cyan());

                    let display_hosts: Vec<SshHostDisplay> =
                        ssh_hosts.iter().map(SshHostDisplay::from).collect();

                    let table = Table::new(display_hosts).to_string();
                    println!("{}", table);
                }

                // Ask if user wants to add a host
                let add_host = Confirm::with_theme(&self.theme)
                    .with_prompt("Would you like to add an SSH host?")
                    .default(true)
                    .interact()?;

                if !add_host {
                    break;
                }

                // Host details
                let host_name: String = Input::with_theme(&self.theme)
                    .with_prompt("Host name (identifier)")
                    .interact_text()?;

                let hostname: String = Input::with_theme(&self.theme)
                    .with_prompt("Hostname or IP address")
                    .interact_text()?;

                let username: String = Input::with_theme(&self.theme)
                    .with_prompt("SSH username")
                    .default("root".into())
                    .interact_text()?;

                let port: u16 = Input::with_theme(&self.theme)
                    .with_prompt("SSH port")
                    .default(22)
                    .interact_text()?;

                let use_identity_file = Confirm::with_theme(&self.theme)
                    .with_prompt("Use identity file for authentication? (If no you will be prompted for the password)")
                    .default(true)
                    .interact()?;

                let mut identity_file: Option<String> = None;
                let mut password: Option<String> = None;
                if use_identity_file {
                    identity_file = Some(
                        Input::with_theme(&self.theme)
                            .with_prompt("Path to identity file")
                            .default("~/.ssh/id_rsa".into())
                            .interact_text()?,
                    );
                } else {
                    let input_password = Input::with_theme(&self.theme)
                        .with_prompt("SSH password")
                        .default("".into())
                        .interact_text()?;
                    password = Some(input_password);
                };

                let is_bastion = Confirm::with_theme(&self.theme)
                    .with_prompt("Is this a bastion/jump host?")
                    .default(false)
                    .interact()?;

                // Add the host to our list
                ssh_hosts.push(SshHost {
                    name: host_name,
                    hostname,
                    username,
                    password,
                    port,
                    identity_file,
                    is_bastion,
                });

                println!("{}", style("✅ SSH host added successfully").green());
            }

            // Additional configuration options
            println!("\n{}", style("⚙️ Additional Configuration").cyan().bold());

            let options = vec!["Enable system monitoring", "Enable automated backups"];
            let defaults = vec![true, true];

            let selections = MultiSelect::with_theme(&self.theme)
                .with_prompt("Select additional services to enable")
                .items(&options)
                .defaults(&defaults)
                .interact()?;

            let enable_monitoring = selections.contains(&0);
            let enable_backups = selections.contains(&1);

            let backup_retention_days = if enable_backups {
                Input::with_theme(&self.theme)
                    .with_prompt("Backup retention period (days)")
                    .default(30)
                    .interact_text()?
            } else {
                7 // Default value if backups are not enabled
            };

            // Create configuration object
            let config = CloudConfig {
                company_name,
                admin_name,
                cloud_name,
                region,
                ssh_hosts,
                enable_monitoring,
                enable_backups,
                backup_retention_days,
            };

            // Save configuration
            println!("\n{}", style("💾 Saving Configuration").cyan().bold());

            if !Path::new(config_dir).exists() {
                fs::create_dir(config_dir).context("Failed to create config directory")?;
            }

            let config_json = serde_json::to_string_pretty(&config)?;
            fs::write(&config_path, config_json).context("Failed to write configuration file")?;

            println!(
                "{}",
                style(format!("✅ Configuration saved to {}", config_path)).green()
            );

            // Summary
            println!("\n{}", style("📊 Configuration Summary").cyan().bold());
            println!("Company: {}", style(&config.company_name).green());
            println!("Admin: {}", style(&config.admin_name).green());
            println!("Cloud Name: {}", style(&config.cloud_name).green());
            println!("Region: {}", style(&config.region).green());
            println!("SSH Hosts: {}", style(config.ssh_hosts.len()).green());
            println!(
                "Monitoring: {}",
                if config.enable_monitoring {
                    style("Enabled").green()
                } else {
                    style("Disabled").yellow()
                }
            );
            println!(
                "Backups: {}",
                if config.enable_backups {
                    style("Enabled").green()
                } else {
                    style("Disabled").yellow()
                }
            );

            if config.enable_backups {
                println!(
                    "Backup Retention: {} days",
                    style(config.backup_retention_days).green()
                );
            }

            config
        };

        // Begin the bootstrapping process
        println!(
            "\n{}",
            style("⚡ Bootstrapping OmniOrchestrator").cyan().bold()
        );
        println!(
            "{}",
            style(format!(
                "Setting up OmniOrchestrator for {} cloud environment",
                config.cloud_name
            ))
            .dim()
        );

        // Check if there are SSH hosts configured
        if config.ssh_hosts.is_empty() {
            println!(
                "{}",
                style("No SSH hosts configured. Cannot bootstrap OmniOrchestrator.").yellow()
            );
            return Ok(());
        }

        // Confirm before proceeding
        let confirm = Confirm::with_theme(&self.theme)
            .with_prompt("Ready to bootstrap OmniOrchestrator on all configured hosts?")
            .default(true)
            .interact()?;

        if !confirm {
            println!("{}", style("Bootstrapping cancelled.").yellow());
            return Ok(());
        }

        // Bootstrap the orchestrator using server-driven approach
        self.bootstrap_orchestrator(&config).await?;

        println!(
            "\n{}",
            style("✨ Environment initialization completed!")
                .green()
                .bold()
        );
        println!(
            "{}",
            style("Your OmniOrchestrator cloud environment is ready.").dim()
        );
        println!(
            "{}",
            style("You can now deploy applications with 'omni deploy'.").dim()
        );

        Ok(())
    }

    async fn bootstrap_orchestrator(&self, config: &CloudConfig) -> Result<()> {
        println!(
            "\n{}",
            style(format!(
                "Initializing platform with {} hosts...",
                config.ssh_hosts.len()
            ))
            .cyan()
        );

        // STEP 1: Initialize the platform by sending configuration to API
        println!("{}", style("Sending configuration to API...").cyan());

        // Make the API call to init the platform with the provided config
        let api_config = CloudConfig {
            company_name: config.company_name.clone(),
            admin_name: config.admin_name.clone(),
            cloud_name: config.cloud_name.clone(),
            region: config.region.clone(),
            ssh_hosts: config.ssh_hosts.clone(),
            enable_monitoring: config.enable_monitoring,
            enable_backups: config.enable_backups,
            backup_retention_days: config.backup_retention_days,
        };

        match self
            .api_client
            .post::<_, ApiResponse>("/platforms/init", &api_config)
            .await
        {
            Err(err) => {
                println!("{}", style("API initialization failed").red().bold());
                println!("{}", style(format!("Error: {:?}", err)).red());
                return Err(anyhow::anyhow!("Failed to initialize platform: {:?}", err));
            }
            Ok(response) => {
                println!("{}", style("Configuration sent successfully ✓").green());
                println!(
                    "{}",
                    style(format!("API response: {}", response.message)).green()
                );
            }
        }

        // STEP 2: Poll for platform status until complete
        let mut all_complete = false;
        let cloud_name = &config.cloud_name;

        println!(
            "\n{}",
            style("Monitoring deployment progress:").cyan().bold()
        );

        let mut prev_lines = 0;
        while !all_complete {
            match self
                .api_client
                .get::<ApiResponse>(&format!("/platforms/{}/status", cloud_name))
                .await
            {
                Err(err) => {
                    println!(
                        "{}",
                        style("Failed to get deployment status: ").red().bold()
                    );
                    println!("{}", style(format!("{:?}", err)).red());
                    // Wait before retrying
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
                Ok(response) => {
                    if response.status == "completed" {
                        all_complete = true;
                        continue;
                    }

                    // Extract host statuses from response data
                    if let Some(data) = response.data {
                        if let Ok(host_statuses) =
                            serde_json::from_value::<Vec<HostDeploymentStatus>>(data)
                        {
                            // Clear previous status lines
                            if prev_lines > 0 {
                                print!("\x1B[{}A\x1B[J", prev_lines);
                            }

                            // Display current status for each host
                            println!("{}", style("Current deployment status:").cyan());
                            for host in &host_statuses {
                                let status_color = match host.status.as_str() {
                                    "completed" => {
                                        style(format!("[✓] {}: {}", host.host, host.current_step))
                                            .green()
                                    }
                                    "in_progress" => {
                                        style(format!("[↻] {}: {}", host.host, host.current_step))
                                            .yellow()
                                    }
                                    "pending" => {
                                        style(format!("[⌛] {}: Waiting", host.host)).dim()
                                    }
                                    "error" => style(format!(
                                        "[✗] {}: Error - {}",
                                        host.host,
                                        host.error.as_ref().unwrap_or(&"Unknown error".to_string())
                                    ))
                                    .red(),
                                    _ => style(format!("[-] {}: {}", host.host, host.current_step))
                                        .dim(),
                                };

                                let progress_bar = if host.status == "completed" {
                                    "██████████".to_string()
                                } else {
                                    let filled = (host.progress as usize) / 10;
                                    let empty = 10 - filled;
                                    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
                                };

                                println!("{} {}% {}", status_color, host.progress, progress_bar);
                            }

                            println!(
                                "Overall: {}",
                                style(format!(
                                    "{}%",
                                    response
                                        .message
                                        .split_whitespace()
                                        .nth(3)
                                        .unwrap_or("0")
                                        .trim_end_matches('%')
                                ))
                                .cyan()
                            );

                            // Track how many lines we printed for clearing next time
                            prev_lines = host_statuses.len() + 2;
                        }
                    }

                    // Wait before polling again
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }

        // STEP 3: Configure network after all hosts are bootstrapped
        println!("\n{}", style("🔄 Configuring cluster networking").cyan());

        match self
            .api_client
            .post::<_, ApiResponse>(&format!("/platforms/{}/network/configure", cloud_name), &())
            .await
        {
            Err(err) => {
                println!("{}", style("Network configuration failed ✗").red().bold());
                println!("{}", style(format!("Error: {:?}", err)).red());
                return Err(anyhow::anyhow!("Failed to configure network: {:?}", err));
            }
            Ok(response) => {
                println!("{}", style("Network configuration initiated ✓").green());
                println!(
                    "{}",
                    style(format!("API response: {}", response.message)).green()
                );

                // Poll status until network configuration is complete
                self.wait_for_process_completion(cloud_name, "network")
                    .await?;
            }
        }

        // STEP 4: Set up monitoring if enabled
        if config.enable_monitoring {
            println!("\n{}", style("📊 Setting up monitoring services").cyan());

            match self
                .api_client
                .post::<_, ApiResponse>(&format!("/platforms/{}/monitoring/setup", cloud_name), &())
                .await
            {
                Err(err) => {
                    println!("{}", style("Monitoring setup failed ✗").red().bold());
                    println!("{}", style(format!("Error: {:?}", err)).red());
                    return Err(anyhow::anyhow!("Failed to setup monitoring: {:?}", err));
                }
                Ok(response) => {
                    println!("{}", style("Monitoring setup initiated ✓").green());
                    println!(
                        "{}",
                        style(format!("API response: {}", response.message)).green()
                    );

                    // Poll status until monitoring setup is complete
                    self.wait_for_process_completion(cloud_name, "monitoring")
                        .await?;
                }
            }
        }

        // STEP 5: Set up backups if enabled
        if config.enable_backups {
            println!("\n{}", style("💾 Configuring backup services").cyan());

            match self
                .api_client
                .post::<_, ApiResponse>(&format!("/platforms/{}/backups/setup", cloud_name), &())
                .await
            {
                Err(err) => {
                    println!("{}", style("Backup setup failed ✗").red().bold());
                    println!("{}", style(format!("Error: {:?}", err)).red());
                    return Err(anyhow::anyhow!("Failed to setup backups: {:?}", err));
                }
                Ok(response) => {
                    println!("{}", style("Backup setup initiated ✓").green());
                    println!(
                        "{}",
                        style(format!("API response: {}", response.message)).green()
                    );

                    // Poll status until backup setup is complete
                    self.wait_for_process_completion(cloud_name, "backups")
                        .await?;
                }
            }
        }

        println!(
            "{}",
            style("\nEnvironment is now fully configured and ready to use! ✓")
                .green()
                .bold()
        );
        Ok(())
    }

    // Generic helper to wait for process completion by polling the status endpoint
    async fn wait_for_process_completion(
        &self,
        cloud_name: &str,
        process_type: &str,
    ) -> Result<()> {
        let mut complete = false;
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 120; // 2 minutes with 1-second intervals

        println!(
            "{}",
            style(format!("Waiting for {} setup to complete...", process_type)).dim()
        );

        while !complete && attempts < MAX_ATTEMPTS {
            attempts += 1;

            match self
                .api_client
                .get::<ApiResponse>(&format!("/platforms/{}/status", cloud_name))
                .await
            {
                Ok(response) => {
                    // Check if the overall platform status is completed
                    if response.status == "completed" {
                        complete = true;
                        println!(
                            "{}",
                            style(format!("{} setup completed ✓", process_type)).green()
                        );
                        break;
                    }

                    // Extract host statuses to check specific process status
                    if let Some(data) = response.data {
                        if let Ok(host_statuses) =
                            serde_json::from_value::<Vec<HostDeploymentStatus>>(data)
                        {
                            // Different processes have different indicators of completion
                            match process_type {
                                "network" => {
                                    // All hosts should have completed network configuration
                                    let network_complete = host_statuses.iter().all(|h| {
                                        h.current_step.contains("Network configuration complete")
                                            || h.current_step.contains("network") && h.completed
                                    });

                                    if network_complete {
                                        complete = true;
                                        println!(
                                            "{}",
                                            style("Network configuration completed ✓").green()
                                        );
                                        break;
                                    }

                                    // Show some progress info
                                    if let Some(host) = host_statuses.first() {
                                        println!(
                                            "{}",
                                            style(format!("Network setup: {}", host.current_step))
                                                .dim()
                                        );
                                    }
                                }
                                "monitoring" => {
                                    // Check if all hosts have the metrics-collector service
                                    let monitoring_ready = host_statuses.iter().all(|h| {
                                        h.services.iter().any(|s| {
                                            s.name == "metrics-collector" && s.status == "Running"
                                        })
                                    });

                                    if monitoring_ready {
                                        complete = true;
                                        println!(
                                            "{}",
                                            style("Monitoring services deployed ✓").green()
                                        );
                                        break;
                                    }

                                    // Show current step from any host that's setting up monitoring
                                    if let Some(host) = host_statuses
                                        .iter()
                                        .find(|h| h.current_step.contains("monitoring"))
                                    {
                                        println!(
                                            "{}",
                                            style(format!(
                                                "Monitoring setup: {}",
                                                host.current_step
                                            ))
                                            .dim()
                                        );
                                    }
                                }
                                "backups" => {
                                    // Check if backup manager is running on bastion hosts
                                    let backups_ready = host_statuses
                                        .iter()
                                        .filter(|h| {
                                            // This is the previous line with error - no longer referencing config
                                            // Just check if the host has a backup-manager service
                                            h.services.iter().any(|s| s.name == "backup-manager")
                                        })
                                        .all(|h| {
                                            h.services.iter().any(|s| {
                                                s.name == "backup-manager" && s.status == "Running"
                                            })
                                        });

                                    if backups_ready {
                                        complete = true;
                                        println!(
                                            "{}",
                                            style("Backup services configured ✓").green()
                                        );
                                        break;
                                    }

                                    // Show backup setup step if available
                                    if let Some(host) = host_statuses
                                        .iter()
                                        .find(|h| h.current_step.contains("backup"))
                                    {
                                        println!(
                                            "{}",
                                            style(format!("Backup setup: {}", host.current_step))
                                                .dim()
                                        );
                                    }
                                }
                                _ => {
                                    // Generic process - just check if all hosts are completed
                                    if host_statuses.iter().all(|h| h.completed) {
                                        complete = true;
                                        println!(
                                            "{}",
                                            style(format!("{} process completed ✓", process_type))
                                                .green()
                                        );
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    println!(
                        "{}",
                        style(format!("Error polling status: {:?}", err)).yellow()
                    );
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        if !complete {
            println!("{}", style(format!("Timed out waiting for {} to complete. The process may still be running on the server.", process_type)).yellow());
        }

        Ok(())
    } // End of function

    // List SSH hosts
    pub async fn list_ssh_hosts(&self) -> Result<()> {
        let config_path = "config/cloud-config.json";

        if !Path::new(config_path).exists() {
            println!(
                "{}",
                style("No cloud configuration found. Run 'omni init' first.").yellow()
            );
            return Ok(());
        }

        let config_json =
            fs::read_to_string(config_path).context("Failed to read configuration file")?;
        let config: CloudConfig =
            serde_json::from_str(&config_json).context("Failed to parse configuration")?;

        if config.ssh_hosts.is_empty() {
            println!(
                "{}",
                style("No SSH hosts configured. Run 'omni init' to add hosts.").yellow()
            );
            return Ok(());
        }

        println!("\n{}", style("📡 Configured SSH Hosts").cyan().bold());
        println!(
            "Cloud: {} ({})",
            style(&config.cloud_name).green(),
            &config.region
        );

        // Get status from API for all hosts
        match self
            .api_client
            .get::<ApiResponse>(&format!("/platforms/{}/status", config.cloud_name))
            .await
        {
            Err(err) => {
                println!("{}", style("Failed to get status from API.").red());
                println!("{}", style(format!("Error: {:?}", err)).dim());
                return Err(anyhow::anyhow!("Failed to get status from API: {:?}", err));
            }
            Ok(response) => {
                if let Some(data) = response.data {
                    if let Ok(host_statuses) =
                        serde_json::from_value::<Vec<HostDeploymentStatus>>(data)
                    {
                        // Display services for each host
                        self.display_service_status(&host_statuses, &config);
                    } else {
                        println!(
                            "{}",
                            style("Failed to parse host status data from API.").red()
                        );
                        return Err(anyhow::anyhow!("Failed to parse host status data"));
                    }
                } else {
                    println!("{}", style("No status data available from API.").yellow());
                    return Err(anyhow::anyhow!("No status data available from API"));
                }
            }
        }

        println!("\n{}", style("💡 Available Commands").cyan().bold());
        println!(
            "- {}: Restart a service",
            style("omni service restart <host> <service>").yellow()
        );
        println!(
            "- {}: View detailed logs",
            style("omni logs <host> <service>").yellow()
        );
        println!(
            "- {}: Trigger immediate backup",
            style("omni backup now").yellow()
        );

        Ok(())
    }

    // Display services status from API data
    fn display_service_status(
        &self,
        host_statuses: &Vec<HostDeploymentStatus>,
        config: &CloudConfig,
    ) {
        let mut services_display = Vec::new();

        for host_status in host_statuses {
            for service in &host_status.services {
                services_display.push(ServiceStatusDisplay {
                    host: host_status.host.clone(),
                    service: service.name.clone(),
                    status: service.status.clone(),
                    uptime: service.uptime.clone().unwrap_or_else(|| "-".to_string()),
                    cpu: service.cpu.clone().unwrap_or_else(|| "-".to_string()),
                    memory: service.memory.clone().unwrap_or_else(|| "-".to_string()),
                });
            }
        }

        if services_display.is_empty() {
            println!("{}", style("No services found.").yellow());
        } else {
            let table = Table::new(services_display).to_string();
            println!("{}", table);
        }

        println!("\n{}", style("🔄 System Information").cyan().bold());
        println!(
            "Monitoring: {}",
            if config.enable_monitoring {
                style("Enabled").green()
            } else {
                style("Disabled").yellow()
            }
        );
        println!(
            "Backups: {}",
            if config.enable_backups {
                style("Enabled").green()
            } else {
                style("Disabled").yellow()
            }
        );
        if config.enable_backups {
            println!(
                "  Retention: {} days",
                style(config.backup_retention_days).green()
            );

            // Get backup information from one of the bastion hosts if available
            for host_status in host_statuses {
                let is_bastion = config
                    .ssh_hosts
                    .iter()
                    .any(|h| h.name == host_status.host && h.is_bastion);

                if is_bastion {
                    if let Some(backup_service) = host_status
                        .services
                        .iter()
                        .find(|s| s.name == "backup-manager")
                    {
                        // In a real implementation, we would extract these dates from service metadata
                        println!("  Last Backup: {}", style("From server data").green());
                        println!("  Next Backup: {}", style("From server data").green());
                        break;
                    }
                }
            }
        }
    }

    // Restart a service via API
    pub async fn restart_service(&self, host_name: &str, service_name: &str) -> Result<()> {
        let config_path = "config/cloud-config.json";
        let config_json =
            fs::read_to_string(config_path).context("Failed to read configuration file")?;
        let config: CloudConfig =
            serde_json::from_str(&config_json).context("Failed to parse configuration")?;

        println!(
            "\n{}",
            style(format!(
                "🔄 Restarting service {} on host {}",
                service_name, host_name
            ))
            .cyan()
            .bold()
        );

        match self
            .api_client
            .post::<_, ApiResponse>(
                &format!(
                    "/platforms/{}/hosts/{}/services/{}/restart",
                    config.cloud_name, host_name, service_name
                ),
                &(),
            )
            .await
        {
            Err(err) => {
                println!("{}", style("Failed to restart service: ").red().bold());
                println!("{}", style(format!("{:?}", err)).red());
                return Err(anyhow::anyhow!("Failed to restart service: {:?}", err));
            }
            Ok(response) => {
                println!("{}", style("Restart request sent successfully ✓").green());
                println!(
                    "{}",
                    style(format!("API response: {}", response.message)).green()
                );

                // Wait for service to restart by polling the host services endpoint
                println!("{}", style("Waiting for service to restart...").dim());

                self.wait_for_service_restart(&config.cloud_name, host_name, service_name)
                    .await?;
            }
        }

        Ok(())
    }

    // Helper to wait for a service to restart
    async fn wait_for_service_restart(
        &self,
        cloud_name: &str,
        host_name: &str,
        service_name: &str,
    ) -> Result<()> {
        let mut service_restarted = false;
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 30;

        while !service_restarted && attempts < MAX_ATTEMPTS {
            attempts += 1;

            match self
                .api_client
                .get::<ApiResponse>(&format!(
                    "/platforms/{}/hosts/{}/services",
                    cloud_name, host_name
                ))
                .await
            {
                Ok(response) => {
                    if let Some(data) = response.data {
                        if let Ok(services) = serde_json::from_value::<Vec<ServiceStatus>>(data) {
                            if let Some(service) = services.iter().find(|s| s.name == service_name)
                            {
                                // Check service status
                                match service.status.as_str() {
                                    "Running" => {
                                        service_restarted = true;
                                        println!(
                                            "{}",
                                            style("Service restarted successfully! ✓")
                                                .green()
                                                .bold()
                                        );
                                        break;
                                    }
                                    "Restarting" => {
                                        println!(
                                            "{}",
                                            style("Service is currently restarting...").yellow()
                                        );
                                    }
                                    status => {
                                        println!(
                                            "{}",
                                            style(format!("Service status: {}", status)).yellow()
                                        );
                                    }
                                }
                            } else {
                                println!(
                                    "{}",
                                    style(format!("Service '{}' not found on host", service_name))
                                        .yellow()
                                );
                            }
                        }
                    }
                }
                Err(err) => {
                    println!(
                        "{}",
                        style(format!("Error checking service status: {:?}", err)).yellow()
                    );
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        if !service_restarted {
            println!("{}", style("Timed out waiting for service to restart. The service may still be restarting.").yellow());
        }

        Ok(())
    }

    // View logs for a specific service
    pub async fn view_service_logs(&self, host_name: &str, service_name: &str) -> Result<()> {
        let config_path = "config/cloud-config.json";
        let config_json =
            fs::read_to_string(config_path).context("Failed to read configuration file")?;
        let config: CloudConfig =
            serde_json::from_str(&config_json).context("Failed to parse configuration")?;

        println!(
            "\n{}",
            style(format!(
                "📜 Logs for service {} on host {}",
                service_name, host_name
            ))
            .cyan()
            .bold()
        );

        match self
            .api_client
            .get::<ApiResponse>(&format!(
                "/platforms/{}/hosts/{}/services/{}/logs",
                config.cloud_name, host_name, service_name
            ))
            .await
        {
            Err(err) => {
                println!("{}", style("Failed to retrieve logs: ").red().bold());
                println!("{}", style(format!("{:?}", err)).red());
                return Err(anyhow::anyhow!("Failed to retrieve logs: {:?}", err));
            }
            Ok(response) => {
                if let Some(data) = response.data {
                    if let Ok(logs) = serde_json::from_value::<Vec<String>>(data) {
                        if logs.is_empty() {
                            println!("{}", style("No logs available for this service.").yellow());
                        } else {
                            println!("\n{}", style("Service Logs:").yellow().bold());
                            for log_line in logs {
                                let formatted_line = if log_line.contains("[INFO]") {
                                    style(log_line).dim()
                                } else if log_line.contains("[WARN]") {
                                    style(log_line).yellow()
                                } else if log_line.contains("[ERROR]") {
                                    style(log_line).red()
                                } else {
                                    style(log_line)
                                };

                                println!("{}", formatted_line);
                            }
                        }
                    } else {
                        println!("{}", style("Failed to parse log data from API.").red());
                        return Err(anyhow::anyhow!("Failed to parse log data"));
                    }
                } else {
                    println!("{}", style("No log data available from API.").yellow());
                    return Err(anyhow::anyhow!("No log data available"));
                }
            }
        }

        println!("\n{}", style("💡 Tip").cyan().bold());
        println!(
            "Use {} to follow logs in real-time",
            style("omni logs <host> <service> --follow").yellow()
        );

        Ok(())
    }

    // Trigger an immediate backup
    pub async fn trigger_backup(&self) -> Result<()> {
        let config_path = "config/cloud-config.json";
        let config_json =
            fs::read_to_string(config_path).context("Failed to read configuration file")?;
        let config: CloudConfig =
            serde_json::from_str(&config_json).context("Failed to parse configuration")?;

        if !config.enable_backups {
            println!(
                "{}",
                style("Backups are not enabled for this cloud environment.").yellow()
            );
            return Ok(());
        }

        println!(
            "\n{}",
            style("💾 Triggering immediate backup").cyan().bold()
        );

        match self
            .api_client
            .post::<_, ApiResponse>(
                &format!("/platforms/{}/backups/trigger", config.cloud_name),
                &(),
            )
            .await
        {
            Err(err) => {
                println!("{}", style("Failed to trigger backup: ").red().bold());
                println!("{}", style(format!("{:?}", err)).red());
                return Err(anyhow::anyhow!("Failed to trigger backup: {:?}", err));
            }
            Ok(response) => {
                println!("{}", style("Backup process initiated ✓").green());
                println!(
                    "{}",
                    style(format!("API response: {}", response.message)).green()
                );

                // Wait for backup to complete by polling the status endpoint
                self.wait_for_backup_completion(&config.cloud_name).await?;
            }
        }

        Ok(())
    }

    // Helper to wait for backup completion
    async fn wait_for_backup_completion(&self, cloud_name: &str) -> Result<()> {
        let mut backup_completed = false;
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 60; // 1 minute timeout

        println!("{}", style("Monitoring backup progress...").dim());

        while !backup_completed && attempts < MAX_ATTEMPTS {
            attempts += 1;

            match self
                .api_client
                .get::<ApiResponse>(&format!("/platforms/{}/backups/status", cloud_name))
                .await
            {
                Ok(response) => {
                    if response.status == "completed" {
                        backup_completed = true;
                        println!(
                            "{}",
                            style("Backup completed successfully! ✓").green().bold()
                        );

                        // Display backup information if available
                        if let Some(data) = response.data {
                            if let Ok(backup_info) =
                                serde_json::from_value::<serde_json::Value>(data)
                            {
                                // Extract and display relevant backup information
                                println!("{}", style("Backup Information:").cyan());
                                if let Some(timestamp) =
                                    backup_info.get("timestamp").and_then(|v| v.as_str())
                                {
                                    println!("Timestamp: {}", style(timestamp).green());
                                }
                                if let Some(size) = backup_info.get("size").and_then(|v| v.as_str())
                                {
                                    println!("Size: {}", style(size).green());
                                }
                            }
                        }

                        break;
                    } else {
                        // Extract and display backup progress information
                        if let Some(data) = response.data {
                            if let Ok(backup_info) =
                                serde_json::from_value::<serde_json::Value>(data)
                            {
                                if let Some(progress) =
                                    backup_info.get("progress").and_then(|v| v.as_u64())
                                {
                                    println!("Backup progress: {}%", style(progress).cyan());
                                }
                                if let Some(current_step) =
                                    backup_info.get("current_step").and_then(|v| v.as_str())
                                {
                                    println!("Current step: {}", style(current_step).dim());
                                }
                            }
                        } else {
                            println!("Waiting for backup progress update...");
                        }
                    }
                }
                Err(err) => {
                    println!(
                        "{}",
                        style(format!("Error checking backup status: {:?}", err)).yellow()
                    );
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        if !backup_completed {
            println!("{}", style("Timed out waiting for backup to complete. The backup may still be in progress.").yellow());
        }

        Ok(())
    }
}
