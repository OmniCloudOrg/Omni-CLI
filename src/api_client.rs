use anyhow::{Result, anyhow};
use reqwest::{self, header::{HeaderMap, HeaderName, HeaderValue}, Client, Method, StatusCode};
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;

pub struct ApiClient {
    client: Client,
    base_url: String,
    headers: HeaderMap,
}

impl ApiClient {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");
            
        Self {
            client,
            base_url: String::from("http://localhost:8002/api/v1"),
            headers,
        }
    }
    
    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }
    
    pub fn with_api_key(mut self, api_key: &str) -> Self {
        self.headers.insert(
            "Authorization", 
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .expect("Invalid API key format")
        );
        self
    }
    
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(
            HeaderName::from_bytes(key.as_bytes()).expect("Invalid header name"), 
            HeaderValue::from_str(value).expect("Invalid header value")
        );
        self
    }
    
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