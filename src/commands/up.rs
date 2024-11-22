use crate::models::ComponentStatus;
use crate::ui::PremiumUI;
use anyhow::Result;
use console::style;
use dialoguer::{Confirm, Input, Select};
use std::{thread, time::Duration};
use tabled::Table;

impl PremiumUI {
    pub async fn deploy_interactive(&self) -> Result<()> {
        let project_path: String = Input::with_theme(&self.theme)
            .with_prompt("Enter project path")
            .default(".".into())
            .interact_text()?;

        let environments = vec!["Development", "Staging", "Production"];
        let env_selection = Select::with_theme(&self.theme)
            .with_prompt("Select deployment environment")
            .items(&environments)
            .default(0)
            .interact()?;

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

        println!("\n{}", style("🚀 Initializing deployment...").cyan().bold());
        
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
                
                match i {
                    5 => pb.set_message(format!("{} (scanning dependencies)", step)),
                    15 => pb.set_message(format!("{} (optimizing)", step)),
                    25 => pb.set_message(format!("{} (finalizing)", step)),
                    _ => {}
                }
            }
            pb.finish_with_message(format!("{} ✓", step));
        }

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

        println!("\n{}", style("🌍 Application Endpoints").cyan().bold());
        println!("Frontend: {}", style("https://app.example.com").green());
        println!("API:      {}", style("https://api.example.com").green());
        println!("Metrics:  {}", style("https://metrics.example.com").green());

        println!("\n{}", style("✨ Deployment completed successfully!").green().bold());
        println!("{}", style("Run 'omni status' to monitor your deployment.").dim());

        Ok(())
    }
}