use crate::models::ComponentStatus;
use anyhow::anyhow;
use tempfile::env::temp_dir;
use crate::ui::PremiumUI;
use anyhow::{Context, Result};
use console::style;
use dialoguer::{Confirm, Input, Select};
use flate2::write::GzEncoder;
use flate2::Compression;
use ignore::WalkBuilder;
use pathdiff;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use serde::Serialize;
use std::{fs::File, path::Path};
use std::{thread, time::Duration};
use tabled::Table;
use tar::Builder;
use tokio::{fs, task};
#[derive(Debug, Serialize, Deserialize)]
pub struct DeployPermissions {
    max_file_count: u64,
}
impl PremiumUI {
    pub async fn deploy_interactive(&self) -> Result<()> {
        // Get project path
        let project_path: String = Input::with_theme(&self.theme)
            .with_prompt("Enter project path")
            .default(".".into())
            .interact_text()?;

        // Validate project path
        if !Path::new(&project_path).exists() {
            println!("{}", style("Error: Project path does not exist.").red());
            return Ok(());
        }

        // Environment selection
        let environments = vec!["Development", "Staging", "Production"];
        let env_selection = Select::with_theme(&self.theme)
            .with_prompt("Select deployment environment")
            .items(&environments)
            .default(0)
            .interact()?;

        // Production confirmation
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

        // Create tarball
        println!("{}", style("🗜️  Creating tarball...").cyan().bold());
        let tarball_path = self
            
            .create_tarball(&project_path)
            
            .await
            .context("Failed to create tarball")?;
        println!("{}", style("🗜️  uploading").cyan().bold());
        let path = std::env::current_dir()?;
        if !path.is_dir() {
            return Err(anyhow!("Invalid project path"));
        }
        let project_path = Path::new(&project_path)
            .canonicalize()
            .expect("Failed to canonicalize path");
        println!("{}", project_path.display());
        let project_name: String = project_path
            .file_name()
            .and_then(|s| s.to_str())
            .map(String::from)
            .expect("Unable to determine folder name"); // Upload tarball
        self.upload_tarball(
            &tarball_path,
            environments[env_selection],
            project_name.as_str(),
        )
        .await
        .context("Failed to upload tarball")?;

        // Clean up tarball
        fs::remove_file(&tarball_path)
            .await
            .context("Failed to clean up tarball")?;

        let steps = [
            ("Analyzing project", 20),
            ("Building containers", 40),
            ("Pushing to registry", 30),
            ("Configuring services", 25),
            ("Starting components", 35),
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
        ])
        .to_string();

        println!("\n{}", style("📊 Deployment Status").cyan().bold());
        println!("{}", status_table);
        println!("\n{}", style("🌍 Application Endpoints").cyan().bold());
        println!("Frontend: {}", style("https://app.example.com").green());
        println!("API:      {}", style("https://api.example.com").green());
        println!("Metrics:  {}", style("https://metrics.example.com").green());
        println!(
            
            "\n{}",
           
            style("✨ Deployment completed successfully!")
                
                .green()
                
                .bold()
        
        );
        println!(
            
            "{}",
           
            style("Run 'omni status' to monitor your deployment.").dim()
        
        );
        Ok(())
    }

    async fn create_tarball(&self, project_path: &str) -> Result<String> {
        // Canonicalize the project path first
        let project_path = fs::canonicalize(project_path)
            .await
            .context("Failed to resolve project path")?;

        // Get the directory name - use the last component of the path
        let project_name = absolute_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| {
                project_path
                    .components()
                    .last()
                    .and_then(|comp| comp.as_os_str().to_str())
                    .unwrap_or("project")
            });

        // Create tarball filename in temp directory
        let temp_dir = temp_dir();
        let tar_gz_path = temp_dir.as_path().join(format!("{}.tar.gz", project_name));

        // Create the tarball file

        let tar_gz_path = format!("{}.tar.gz", project_name);

        // Create a file for the tarball
        let tar_gz = File::create(&tar_gz_path)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let builder = std::sync::Arc::new(std::sync::Mutex::new(Builder::new(enc)));

        // Count total files first
        let mut total_files = 0;
        let walker = WalkBuilder::new(&project_path)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();

        for entry in walker.filter_map(|e| e.ok()) {
            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                total_files += 1;
            }
        }
        let client = reqwest::Client::new();
        let max_file_count = client
            .post("http://localhost:3030/deploy/permissions")
            .send()
            .await;
        match max_file_count {
            Ok(response) => match response.text().await {
                Ok(json_str) => {
                    let json: Result<DeployPermissions, _> = serde_json::from_str(&json_str);
                    let permissions = json.unwrap_or_else(|_| {
                        println!("{}", style(""));
                        std::process::exit(0)
                    });
                    if total_files > permissions.max_file_count {
                        let too_many_files: i64 =
                            total_files as i64 - permissions.max_file_count as i64;
                        println!("{}",style(format!("The server had denied your deployment request. Your project contains {} too many files. ({}/{})",too_many_files,total_files,permissions.max_file_count)).red());
                        std::process::exit(0);
                    }
                }
                Err(e) => {
                    eprintln!("{}", style(format!("Deployment failed: {e}",)).red().bold());
                    std::process::exit(0);
                }
            },
            Err(e) => {
                eprintln!("{}", style(format!("Deployment failed: {e}",)).red().bold());
                std::process::exit(0);
            }
        }
        if total_files > 5000 {
            // let should_continue = Input::with_theme(&self.theme)
            //     .with_prompt("Enter project path")
            //     .interact()?;
            let path_str = format!("{}", project_path.display());
            let current_path_str = style(format!(
                "You are about to upload the entire of {}",
                path_str
            ))
            .yellow()
            .bold()
            .underlined();
            let prompt = format!("Your project contains more than 5000 files.
Are you sure you would like to deploy it? This make take significant amounts of time and space on your machine.\n{}",
                current_path_str);
            let confirm = dialoguer::Confirm::with_theme(&self.theme)
                .default(false)
                .with_prompt(prompt)
                .report(false)
                .show_default(true)
                .interact()?;
            if !confirm {
                println!("{}", style("Canceling upload operation").bold().blue());
                std::process::exit(0)
            }
        }

        let pb = self.create_progress_bar(total_files, "Creating tarball");
        pb.set_message("Initializing tarball creation");

        // Process files
        let mut files_processed = 0;
        let walker = WalkBuilder::new(&project_path)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();

        for entry in walker.filter_map(|e| e.ok()) {
            if let Some(file_type) = entry.file_type() {
                let entry_path = entry.path().to_path_buf();

                // Convert the entry path to a relative path using path difference
                let relative_path = pathdiff::diff_paths(&entry_path, &project_path)
                    .ok_or_else(|| anyhow::anyhow!("Failed to compute relative path"))?;

                // Skip root directory
                if relative_path.as_os_str().is_empty() {
                    continue;
                }

                if file_type.is_dir() {
                    pb.set_message(format!("Adding directory: {}", relative_path.display()));

                    let builder = std::sync::Arc::clone(&builder);
                    let relative_path = relative_path.clone();

                    task::spawn_blocking(move || -> Result<()> {
                        let mut builder = builder.lock().unwrap();
                        let mut header = tar::Header::new_ustar();
                        header.set_entry_type(tar::EntryType::Directory);
                        header.set_mode(0o755);
                        header.set_size(0);
                        builder.append_data(&mut header, relative_path, &[][..])?;
                        Ok(())
                    })
                    .await??;
                } else if file_type.is_file() {
                    let file_contents = fs::read(&entry_path)
                        .await
                        .with_context(|| format!("Failed to read file: {:?}", entry_path))?;

                    let builder = std::sync::Arc::clone(&builder);
                    let relative_path_clone = relative_path.clone();

                    task::spawn_blocking(move || -> Result<()> {
                        let mut builder = builder.lock().unwrap();
                        let mut header = tar::Header::new_ustar();
                        header.set_size(file_contents.len() as u64);
                        header.set_mode(0o644);
                        builder.append_data(
                            &mut header,
                            relative_path_clone,
                            &file_contents[..],
                        )?;
                        Ok(())
                    })
                    .await??;

                    files_processed += 1;
                    pb.set_position(files_processed);
                    pb.set_message(format!("Adding file: {}", relative_path.display()));
                }

                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }

        // Finalize the tarball
        pb.set_message("Finalizing tarball");

        task::spawn_blocking(move || -> Result<()> {
            let mut builder = builder.lock().unwrap();
            builder.finish()?;
            Ok(())
        })
        .await??;

        pb.finish_with_message("Tarball created successfully ✓");
        Ok(tar_gz_path)
        let mut tar = Builder::new(enc); // Add the directory contents to the tarball
        tar.append_dir_all(".", project_path)
            .context("Failed to add directory contents to tarball")?;

        // Finish creating the tarball
        tar.into_inner()
            .context("Failed to finish tarball creation")?
            .finish()
            .context("Failed to finish compression")?;

        Ok(tar_gz_path.to_string_lossy().to_string())
    }
    async fn upload_tarball(
        &self,
        tarball_path: &str,
        environment: &str,
        name: &str,
    ) -> Result<()> {
        let client = reqwest::Client::new();
        let path = PathBuf::from(tarball_path);
        if !path.is_file() {
            return Err(anyhow!("Path is not a file"));
        }
        let uuid = uuid::Uuid::new_v4();
        let uuid_str = format!("u-{}", uuid.to_string());

        let api_url = match environment {
            // "Production" => "http://localhost:3030/upload".to_string(),
            // "Staging" => "https://staging-api.example.com/v1/deploy".to_string(),
            _ => format!("http://100.120.68.15:8000/api/v1/apps/{name}/releases/{uuid_str}/upload"),
        };
        println!("Project name: {name}");
        println!("Uploading project with post request: {}", api_url);
        let file_content = fs::read(tarball_path).await?;

        // Create the part with the correct field name "media" to match server expectations
        let part = Part::bytes(file_content)
            .file_name(name.to_string())
            .mime_str("application/gzip")?;

        // Use "media" as the field name to match the server's expected field
        let form = Form::new()
            .part("media", part)
            .text("environment", environment.to_string());

        let pb = self.create_progress_bar(100, "Uploading project");


        let response = client
            .post(api_url)
                        //.bearer_auth(&self.config.api_token)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            pb.abandon_with_message("Upload failed!");
            anyhow::bail!(
                "Failed to upload tarball: {} - {}",
                response.status(),
                response
                    .text()
                    .await
                    .unwrap_or_else(|_| "No error message".to_string())
            );
        }

        pb.finish_with_message("Upload completed successfully ✓");
        Ok(())
    }
}

