use anyhow::Result;
use console::{style, Term};
use dialoguer::theme::ColorfulTheme;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use spinners::{Spinner, Spinners};
use std::{thread, time::Duration};

const LOGO: &str = r#"
   ____                  _          ______ __                  __
  / __ \____ ___  ____  (_)        / ____// / ___  __  __ ____/ /
 / / / / __ `__ \/ __ \/ /  ____  / /    / // __ \/ / / // __  / 
/ /_/ / / / / / / / / / / /____/ / /___ / // /_/ / /_/ // /_/ /  
\____/_/ /_/ /_/_/ /_/_/         \____//_/ \____/\__,_/ \__,_/   
"#;

// Gradient colors (simulated with different shades of blue)
const GRADIENT_COLORS: [&str; 5] = ["#00c6ff", "#0072ff", "#0057ff", "#0053d4", "#00c6ff"];

pub struct PremiumUI {
    pub term: Term,
    pub multi_progress: MultiProgress,
    pub theme: ColorfulTheme,
}

impl PremiumUI {
    pub fn new() -> Self {
        Self {
            term: Term::stdout(),
            multi_progress: MultiProgress::new(),
            theme: ColorfulTheme::default(),
        }
    }

    pub fn display_welcome(&self) -> Result<()> {
        self.term.clear_screen()?;
        
        // Print logo with a simulated gradient effect
        self.print_gradient_logo();
        
        // Print boxed CLI information that matches the screenshot
        self.print_info_box();
        
        Ok(())
    }
    
    fn print_gradient_logo(&self) {
        // Split the logo into lines
        let logo_lines: Vec<&str> = LOGO.trim_matches('\n').split('\n').collect();
        
        for (i, line) in logo_lines.iter().enumerate() {
            // Use modulo to cycle through colors for a gradient-like effect
            let color_index = i % GRADIENT_COLORS.len();
            println!("{}", style(line).color256(39 + color_index as u8).bold());
        }
        println!();
    }
    
    fn print_info_box(&self) {
        println!("┌{:─^53}┐", "");
        println!("│ {}{}│", 
            style("OMNICLOUD CLI").bold().cyan(),
            " ".repeat(39));
        println!("│ {}{}│", 
            style("Version 1.0.0").dim(),
            " ".repeat(39));
        println!("│{}│", " ".repeat(53));
        println!("│ {} Type {} to see available commands{}│", 
            style("→").cyan(), 
            style("omni help").green(),
            " ".repeat(10));
        println!("│ {} Documentation: {}{}│", 
            style("→").cyan(), 
            style("https://docs.omniforge.io").cyan(),
            " ".repeat(10));
        println!("│ {} Support: {}{}│", 
            style("→").cyan(), 
            style("support@omniforge.io").cyan(),
            " ".repeat(21));
        println!("└{:─^53}┘", "");
        println!();
    }
    
    fn print_status_indicators(&self) {
        // These are removed based on the screenshot
    }
    
    fn show_initializing_spinner(&self) -> Result<()> {
        // Removed the initialization spinner as requested
        Ok(())
    }

    pub fn create_spinner(&self, message: &str) -> Spinner {
        Spinner::with_timer(Spinners::Dots12, message.into())
    }

    pub fn create_progress_bar(&self, len: u64, message: &str) -> ProgressBar {
        let pb = self.multi_progress.add(ProgressBar::new(len));
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("=>-"));
        pb.set_message(message.to_string());
        pb
    }
    
    // New method for displaying cloud-themed progress
    pub fn deploy_with_progress(&self, steps: u64) -> Result<()> {
        let pb = self.create_progress_bar(steps, "Deploying to cloud");
        
        for i in 0..steps {
            pb.inc(1);
            
            // Add different messages based on progress
            match i {
                1 => pb.set_message("Initializing containers...".to_string()),
                3 => pb.set_message("Configuring network...".to_string()),
                5 => pb.set_message("Launching services...".to_string()),
                7 => pb.set_message("Almost there...".to_string()),
                _ => {}
            }
            
            thread::sleep(Duration::from_millis(300));
        }
        
        pb.finish_with_message("Deployment complete!".to_string());
        Ok(())
    }
}