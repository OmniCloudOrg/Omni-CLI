use anyhow::{Result, anyhow};
use reqwest::{self, header::{HeaderMap, HeaderName, HeaderValue}, Client, Method, StatusCode};
use serde::{Serialize, de::DeserializeOwned, Deserialize};
use std::time::Duration;
use std::{fs, io, path::{PathBuf, Path}};
use std::collections::HashMap;
use dirs;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub base_url: String,
    pub timeout_seconds: u64,
    // Store arbitrary key-value pairs for different parts of the app
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,
}

pub struct ApiClient {
    pub client: Client,
    pub base_url: String,
    pub headers: HeaderMap,
    pub config_path: Option<PathBuf>,
    pub config: AppConfig,
}

impl ApiClient {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        
        let app_name = env!("CARGO_PKG_NAME");
        
        // Initialize with defaults
        let mut config = AppConfig::default();
        config.base_url = String::from("http://localhost:8002/api/v1");
        config.timeout_seconds = 30;
        
        let config_path = dirs::config_dir().map(|config_dir| {
            let app_config_dir = config_dir.join(app_name);
            let config_file = app_config_dir.join("config.json");
            
            // Ensure the app config directory exists
            Self::ensure_config_dir(&app_config_dir);
            
            // Load config if it exists, otherwise create default
            match Self::load_config(&config_file) {
                Ok(loaded_config) => {
                    config = loaded_config;
                    println!("Loaded configuration from {:?}", config_file);
                },
                Err(_) => {
                    // Write default config
                    if let Err(err) = Self::write_config(&config_file, &config) {
                        eprintln!("Failed to write default config: {}", err);
                    } else {
                        println!("Created default config at {:?}", config_file);
                    }
                }
            }
            
            config_file
        });

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to build HTTP client");
            
        Self {
            client,
            base_url: config.base_url.clone(),
            headers,
            config_path,
            config,
        }
    }
    
    // Helper methods for configuration management
    fn ensure_config_dir(dir: &Path) {
        if !dir.exists() {
            if let Err(err) = fs::create_dir_all(dir) {
                eprintln!("Failed to create config directory: {}", err);
            }
        }
    }
    
    fn load_config(path: &Path) -> Result<AppConfig> {
        if !path.exists() {
            return Err(anyhow!("Config file doesn't exist"));
        }
        
        let content = fs::read_to_string(path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }
    
    // Renamed to avoid collision with instance method
    fn write_config(path: &Path, config: &AppConfig) -> io::Result<()> {
        let json = serde_json::to_string_pretty(config)?;
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(path, json)
    }
    
    // Key-value storage methods
    
    /// Get a setting value by key
    pub fn get_setting<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.config.settings.get(key)
            .and_then(|value| serde_json::from_value(value.clone()).ok())
    }
    
    /// Get a setting with default fallback
    pub fn get_setting_or<T: DeserializeOwned>(&self, key: &str, default: T) -> T {
        self.get_setting(key).unwrap_or(default)
    }
    
    /// Set a setting value
    pub fn set_setting<T: Serialize>(&mut self, key: &str, value: T) -> Result<()> {
        let json_value = serde_json::to_value(value)?;
        self.config.settings.insert(key.to_string(), json_value);
        
        // Save the updated config
        self.save_config()
    }
    
    /// Remove a setting
    pub fn remove_setting(&mut self, key: &str) -> bool {
        let removed = self.config.settings.remove(key).is_some();
        if removed {
            // Only save if something was actually removed
            let _ = self.save_config();
        }
        removed
    }
    
    /// Save the current configuration to disk
    pub fn save_config(&self) -> Result<()> {
        if let Some(config_path) = &self.config_path {
            Self::write_config(config_path, &self.config)
                .map_err(|e| anyhow!("Failed to save config: {}", e))?;
            Ok(())
        } else {
            Err(anyhow!("No config path available"))
        }
    }
    
    /// Get a section of settings with a common prefix
    pub fn get_settings_section(&self, prefix: &str) -> HashMap<String, serde_json::Value> {
        self.config.settings.iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
    
    // Builder methods
    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self.config.base_url = base_url.to_string();
        // Ignore errors during chain building
        let _ = self.save_config();
        self
    }
    
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.config.timeout_seconds = seconds;
        // Recreate client with new timeout
        self.client = Client::builder()
            .timeout(Duration::from_secs(seconds))
            .build()
            .expect("Failed to build HTTP client");
        let _ = self.save_config();
        self
    }
    
    pub fn with_api_key(mut self, api_key: &str) -> Self {
        self.headers.insert(
            "Authorization", 
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .expect("Invalid API key format")
        );
        // Store API key in settings
        let _ = self.set_setting("api_key", api_key);
        self
    }
    
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(
            HeaderName::from_bytes(key.as_bytes()).expect("Invalid header name"), 
            HeaderValue::from_str(value).expect("Invalid header value")
        );
        self
    }
    
    // HTTP Request methods (unchanged)
    pub async fn request<T, U>(&self, method: Method, endpoint: &str, body: Option<&T>) -> Result<U> 
    where 
        T: Serialize + ?Sized,
        U: DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, endpoint);
        
        let mut request = self.client.request(method, &url);
        request = request.headers(self.headers.clone());
        
        if let Some(data) = body {
            request = request.json(data);
        }
        
        let response = request.send().await?;
        
        match response.status() {
            StatusCode::OK | StatusCode::CREATED | StatusCode::ACCEPTED => {
                let data = response.json::<U>().await?;
                Ok(data)
            },
            status => {
                let error_text = response.text().await?;
                Err(anyhow!("API error: {} - {}", status, error_text))
            }
        }
    }
    
    // Convenience methods for common HTTP verbs
    pub async fn get<U>(&self, endpoint: &str) -> Result<U> 
    where 
        U: DeserializeOwned,
    {
        self.request::<(), U>(Method::GET, endpoint, None).await
    }
    
    pub async fn post<T, U>(&self, endpoint: &str, body: &T) -> Result<U> 
    where 
        T: Serialize + ?Sized,
        U: DeserializeOwned,
    {
        self.request::<T, U>(Method::POST, endpoint, Some(body)).await
    }
    
    pub async fn put<T, U>(&self, endpoint: &str, body: &T) -> Result<U> 
    where 
        T: Serialize + ?Sized,
        U: DeserializeOwned,
    {
        self.request::<T, U>(Method::PUT, endpoint, Some(body)).await
    }
    
    pub async fn delete<U>(&self, endpoint: &str) -> Result<U> 
    where 
        U: DeserializeOwned,
    {
        self.request::<(), U>(Method::DELETE, endpoint, None).await
    }
    
    pub async fn patch<T, U>(&self, endpoint: &str, body: &T) -> Result<U> 
    where 
        T: Serialize + ?Sized,
        U: DeserializeOwned,
    {
        self.request::<T, U>(Method::PATCH, endpoint, Some(body)).await
    }
}