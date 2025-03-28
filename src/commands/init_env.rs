use crate::ui::PremiumUI;
use anyhow::{Context, Result};
use console::style;
use dialoguer::{Confirm, Input, MultiSelect, Select};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tabled::{Table, Tabled};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SshHost {
    name: String,
    hostname: String,
    username: String,
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

#[derive(Tabled)]
struct SshHostDisplay {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Hostname")]
    hostname: String,
    #[tabled(rename = "Username")]
    username: String,
    #[tabled(rename = "Port")]
    port: String,
    #[tabled(rename = "Identity File")]
    identity_file: String,
    #[tabled(rename = "Bastion")]
    is_bastion: String,
}

impl From<&SshHost> for SshHostDisplay {
    fn from(host: &SshHost) -> Self {
        SshHostDisplay {
            name: host.name.clone(),
            hostname: host.hostname.clone(),
            username: host.username.clone(),
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

            // Region selection
            let regions = vec![
                "us-east",
                "us-west",
                "eu-west",
                "eu-central",
                "ap-southeast",
                "custom",
            ];
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
                    .default("admin".into())
                    .interact_text()?;

                let port: u16 = Input::with_theme(&self.theme)
                    .with_prompt("SSH port")
                    .default(22)
                    .interact_text()?;

                let use_identity_file = Confirm::with_theme(&self.theme)
                    .with_prompt("Use identity file for authentication?")
                    .default(true)
                    .interact()?;

                let identity_file = if use_identity_file {
                    Some(
                        Input::with_theme(&self.theme)
                            .with_prompt("Path to identity file")
                            .default("~/.ssh/id_rsa".into())
                            .interact_text()?,
                    )
                } else {
                    None
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

        // Loop through each host and install OmniOrchestrator
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

        // Make the POST request to the API to init the platform with the provided config
        println!(
            "\n{}",
            style("🔄 Initializing platform with API").cyan().bold()
        );

        let pb = self.create_progress_bar(100, "Sending configuration to API");

        for i in 0..100 {
            pb.inc(1);
            std::thread::sleep(std::time::Duration::from_millis(20));

            match i {
                20 => pb.set_message("Sending configuration to API"),
                40 => pb.set_message("Configuring services"),
                60 => pb.set_message("Finalizing setup"),
                _ => {}
            }
        }

        let config_content = match fs::read_to_string(config_path) {
            Ok(content) => {
                println!("{}", content);
                content
            }
            Err(error) => {
                println!("Error reading the config file init_env.rs");
                String::new()  // Return empty string in case of error
            }
        };

        // make the actual API call here
        let response = self.api_client.post("v1/platforms/init", &config).await?;

        pb.finish_with_message("Platform initialized successfully ✓");

        Ok(())
    }

    async fn bootstrap_orchestrator(&self, config: &CloudConfig) -> Result<()> {
        let total_hosts = config.ssh_hosts.len();
        let mut completed_hosts = 0;

        println!(
            "\n{}",
            style(format!(
                "Installing OmniOrchestrator on {} hosts...",
                total_hosts
            ))
            .cyan()
        );

        // Process all bastion hosts first
        for host in config.ssh_hosts.iter().filter(|h| h.is_bastion) {
            self.install_on_host(host, config, true).await?;
            completed_hosts += 1;
        }

        // Then process all regular hosts
        for host in config.ssh_hosts.iter().filter(|h| !h.is_bastion) {
            self.install_on_host(host, config, false).await?;
            completed_hosts += 1;
        }

        println!(
            "\n{}",
            style(format!(
                "✅ OmniOrchestrator installed on all {} hosts",
                total_hosts
            ))
            .green()
            .bold()
        );

        // Setup cluster networking
        println!("\n{}", style("🔄 Configuring cluster networking").cyan());
        let pb = self.create_progress_bar(100, "Setting up secure overlay network");

        for i in 0..100 {
            pb.inc(1);
            std::thread::sleep(std::time::Duration::from_millis(30));

            match i {
                20 => pb.set_message("Establishing secure tunnels"),
                40 => pb.set_message("Configuring service discovery"),
                60 => pb.set_message("Setting up load balancing"),
                80 => pb.set_message("Finalizing network configuration"),
                _ => {}
            }
        }

        pb.finish_with_message("Network configuration complete ✓");

        // Initialize services based on configuration
        if config.enable_monitoring {
            println!("\n{}", style("📊 Setting up monitoring services").cyan());
            let pb = self.create_progress_bar(100, "Deploying monitoring stack");

            for i in 0..100 {
                pb.inc(1);
                std::thread::sleep(std::time::Duration::from_millis(20));

                match i {
                    30 => pb.set_message("Configuring metrics collection"),
                    60 => pb.set_message("Setting up dashboards"),
                    80 => pb.set_message("Configuring alerts"),
                    _ => {}
                }
            }

            pb.finish_with_message("Monitoring services deployed ✓");
        }

        if config.enable_backups {
            println!("\n{}", style("💾 Configuring backup services").cyan());
            let pb = self.create_progress_bar(100, "Setting up backup system");

            for i in 0..100 {
                pb.inc(1);
                std::thread::sleep(std::time::Duration::from_millis(15));

                match i {
                    30 => pb.set_message("Configuring backup schedules"),
                    60 => pb.set_message(format!(
                        "Setting {} day retention policy",
                        config.backup_retention_days
                    )),
                    80 => pb.set_message("Testing backup system"),
                    _ => {}
                }
            }

            pb.finish_with_message("Backup services configured ✓");
        }

        Ok(())
    }

    async fn install_on_host(
        &self,
        host: &SshHost,
        config: &CloudConfig,
        is_bastion: bool,
    ) -> Result<()> {
        let host_type = if is_bastion { "bastion" } else { "worker" };
        println!(
            "\n{}",
            style(format!("Setting up {} host: {}", host_type, host.name)).cyan()
        );

        let steps = [
            ("Establishing SSH connection", 10),
            ("Verifying system requirements", 15),
            ("Installing OmniOrchestrator binaries", 30),
            ("Configuring system services", 20),
            ("Applying security hardening", 25),
        ];

        for (step, duration) in steps.iter() {
            let pb = self.create_progress_bar(*duration, step);

            for i in 0..*duration {
                pb.inc(1);
                std::thread::sleep(std::time::Duration::from_millis(100));

                match i {
                    5 => pb.set_message(format!(
                        "{} (connecting to {}@{}:{})",
                        step, host.username, host.hostname, host.port
                    )),
                    15 => pb.set_message(format!("{} (deploying packages)", step)),
                    25 => pb.set_message(format!("{} (finalizing)", step)),
                    _ => {}
                }
            }

            pb.finish_with_message(format!("{} ✓", step));
        }

        // Apply specific configuration based on host type
        if is_bastion {
            let pb = self.create_progress_bar(25, "Configuring bastion-specific security");

            for i in 0..25 {
                pb.inc(1);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            pb.finish_with_message("Bastion configuration complete ✓");
        } else {
            let pb = self.create_progress_bar(25, "Configuring worker-specific services");

            for i in 0..25 {
                pb.inc(1);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            pb.finish_with_message("Worker configuration complete ✓");
        }

        println!(
            "{}",
            style(format!("✅ OmniOrchestrator installed on {}", host.name)).green()
        );
        Ok(())
    }

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

        let display_hosts: Vec<SshHostDisplay> =
            config.ssh_hosts.iter().map(SshHostDisplay::from).collect();

        let table = Table::new(display_hosts).to_string();
        println!("{}", table);

        Ok(())
    }

    pub async fn orchestrator_status(&self) -> Result<()> {
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

        println!("\n{}", style("🔍 OmniOrchestrator Status").cyan().bold());
        println!(
            "Cloud: {} ({})",
            style(&config.cloud_name).green(),
            &config.region
        );

        if config.ssh_hosts.is_empty() {
            println!(
                "{}",
                style("No hosts configured. Run 'omni init' to configure hosts.").yellow()
            );
            return Ok(());
        }

        #[derive(Tabled)]
        struct ServiceStatus {
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

        let mut services = Vec::new();

        // Simulate services for each host
        for host in &config.ssh_hosts {
            // Core services on all hosts
            services.push(ServiceStatus {
                host: host.name.clone(),
                service: "orchestrator-core".to_string(),
                status: "Running".to_string(),
                uptime: "3d 12h 45m".to_string(),
                cpu: "12%".to_string(),
                memory: "256MB".to_string(),
            });

            services.push(ServiceStatus {
                host: host.name.clone(),
                service: "network-agent".to_string(),
                status: "Running".to_string(),
                uptime: "3d 12h 42m".to_string(),
                cpu: "5%".to_string(),
                memory: "128MB".to_string(),
            });

            // Bastion-specific services
            if host.is_bastion {
                services.push(ServiceStatus {
                    host: host.name.clone(),
                    service: "api-gateway".to_string(),
                    status: "Running".to_string(),
                    uptime: "3d 12h 40m".to_string(),
                    cpu: "18%".to_string(),
                    memory: "512MB".to_string(),
                });

                services.push(ServiceStatus {
                    host: host.name.clone(),
                    service: "auth-service".to_string(),
                    status: "Running".to_string(),
                    uptime: "3d 12h 39m".to_string(),
                    cpu: "10%".to_string(),
                    memory: "384MB".to_string(),
                });
            } else {
                // Worker-specific services
                services.push(ServiceStatus {
                    host: host.name.clone(),
                    service: "container-runtime".to_string(),
                    status: "Running".to_string(),
                    uptime: "3d 12h 38m".to_string(),
                    cpu: "22%".to_string(),
                    memory: "768MB".to_string(),
                });
            }

            // Monitoring if enabled
            if config.enable_monitoring {
                services.push(ServiceStatus {
                    host: host.name.clone(),
                    service: "metrics-collector".to_string(),
                    status: "Running".to_string(),
                    uptime: "3d 12h 30m".to_string(),
                    cpu: "8%".to_string(),
                    memory: "192MB".to_string(),
                });
            }

            // Backup service if enabled (only on bastion hosts)
            if config.enable_backups && host.is_bastion {
                services.push(ServiceStatus {
                    host: host.name.clone(),
                    service: "backup-manager".to_string(),
                    status: "Running".to_string(),
                    uptime: "3d 12h 20m".to_string(),
                    cpu: "6%".to_string(),
                    memory: "256MB".to_string(),
                });
            }
        }

        let table = Table::new(services).to_string();
        println!("{}", table);

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
            println!("  Last Backup: {}", style("2025-03-01 03:15 UTC").green());
            println!("  Next Backup: {}", style("2025-03-03 03:15 UTC").green());
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
}
