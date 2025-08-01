use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use serde::{Deserialize, Serialize};
use tokio::{sync::mpsc, time::interval};
use log::{debug, error, info, warn};
use url::Url;

/// External data source configuration
#[derive(Clone, Debug, Deserialize)]
pub struct ExternalDataSource {
    /// Unique identifier for this data source
    pub id: String,
    
    /// Type of data source
    #[serde(flatten)]
    pub source_type: ExternalDataSourceType,
    
    /// Refresh interval in seconds
    #[serde(default = "ExternalDataSource::default_refresh_interval")]
    pub refresh_interval_secs: u64,
    
    /// Cache TTL in seconds
    #[serde(default = "ExternalDataSource::default_cache_ttl")]
    pub cache_ttl_secs: u64,
    
    /// Enable automatic refresh
    #[serde(default = "ExternalDataSource::default_auto_refresh")]
    pub auto_refresh: bool,
    
    /// Retry configuration
    #[serde(default)]
    pub retry_config: RetryConfig,
}

impl ExternalDataSource {
    fn default_refresh_interval() -> u64 {
        300 // 5 minutes
    }
    
    fn default_cache_ttl() -> u64 {
        600 // 10 minutes
    }
    
    fn default_auto_refresh() -> bool {
        true
    }
}

/// Types of external data sources
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ExternalDataSourceType {
    /// HTTP/HTTPS endpoint
    #[serde(rename = "http")]
    Http(HttpDataSource),
    
    /// File on disk
    #[serde(rename = "file")]
    File(FileDataSource),
    
    /// Database query
    #[serde(rename = "database")]
    Database(DatabaseDataSource),
    
    /// Redis cache
    #[serde(rename = "redis")]
    Redis(RedisDataSource),
    
    /// Another RIB unit
    #[serde(rename = "rib")]
    Rib(RibDataSource),
}

/// HTTP data source configuration
#[derive(Clone, Debug, Deserialize)]
pub struct HttpDataSource {
    /// URL to fetch data from
    pub url: Url,
    
    /// HTTP method
    #[serde(default = "HttpDataSource::default_method")]
    pub method: String,
    
    /// HTTP headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    
    /// Request body (for POST/PUT)
    pub body: Option<String>,
    
    /// Request timeout in seconds
    #[serde(default = "HttpDataSource::default_timeout")]
    pub timeout_secs: u64,
    
    /// Expected content type
    pub content_type: Option<String>,
    
    /// Authentication configuration
    pub auth: Option<HttpAuth>,
}

impl HttpDataSource {
    fn default_method() -> String {
        "GET".to_string()
    }
    
    fn default_timeout() -> u64 {
        30
    }
}

/// HTTP authentication configuration
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum HttpAuth {
    #[serde(rename = "basic")]
    Basic { username: String, password: String },
    
    #[serde(rename = "bearer")]
    Bearer { token: String },
    
    #[serde(rename = "api_key")]
    ApiKey { header: String, value: String },
}

/// File data source configuration
#[derive(Clone, Debug, Deserialize)]
pub struct FileDataSource {
    /// Path to the file
    pub path: std::path::PathBuf,
    
    /// File format
    #[serde(default = "FileDataSource::default_format")]
    pub format: FileFormat,
    
    /// Watch for file changes
    #[serde(default = "FileDataSource::default_watch")]
    pub watch: bool,
}

impl FileDataSource {
    fn default_format() -> FileFormat {
        FileFormat::Json
    }
    
    fn default_watch() -> bool {
        true
    }
}

/// File format for external data
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Json,
    Yaml,
    Toml,
    Csv,
    Text,
}

/// Database data source configuration
#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseDataSource {
    /// Database connection string
    pub connection_string: String,
    
    /// SQL query to execute
    pub query: String,
    
    /// Query parameters
    #[serde(default)]
    pub parameters: HashMap<String, String>,
    
    /// Connection pool size
    #[serde(default = "DatabaseDataSource::default_pool_size")]
    pub pool_size: u32,
}

impl DatabaseDataSource {
    fn default_pool_size() -> u32 {
        5
    }
}

/// Redis data source configuration
#[derive(Clone, Debug, Deserialize)]
pub struct RedisDataSource {
    /// Redis connection URL
    pub url: String,
    
    /// Redis key pattern or specific key
    pub key: String,
    
    /// Redis command to execute
    #[serde(default = "RedisDataSource::default_command")]
    pub command: String,
    
    /// Database number
    #[serde(default)]
    pub database: u8,
}

impl RedisDataSource {
    fn default_command() -> String {
        "GET".to_string()
    }
}

/// RIB data source configuration
#[derive(Clone, Debug, Deserialize)]
pub struct RibDataSource {
    /// RIB unit name to query
    pub rib_name: String,
    
    /// Query type
    pub query_type: RibQueryType,
    
    /// Query parameters
    #[serde(default)]
    pub parameters: HashMap<String, String>,
}

/// Types of RIB queries
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RibQueryType {
    /// Query for specific prefix
    Prefix,
    /// Query for prefixes from specific ASN
    Asn,
    /// Query for prefixes with specific community
    Community,
    /// Custom query
    Custom,
}

/// Retry configuration for external data sources
#[derive(Clone, Debug, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries
    #[serde(default = "RetryConfig::default_max_retries")]
    pub max_retries: u32,
    
    /// Initial delay between retries in milliseconds
    #[serde(default = "RetryConfig::default_initial_delay_ms")]
    pub initial_delay_ms: u64,
    
    /// Maximum delay between retries in milliseconds
    #[serde(default = "RetryConfig::default_max_delay_ms")]
    pub max_delay_ms: u64,
    
    /// Backoff multiplier
    #[serde(default = "RetryConfig::default_backoff_multiplier")]
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: Self::default_max_retries(),
            initial_delay_ms: Self::default_initial_delay_ms(),
            max_delay_ms: Self::default_max_delay_ms(),
            backoff_multiplier: Self::default_backoff_multiplier(),
        }
    }
}

impl RetryConfig {
    fn default_max_retries() -> u32 {
        3
    }
    
    fn default_initial_delay_ms() -> u64 {
        1000
    }
    
    fn default_max_delay_ms() -> u64 {
        10000
    }
    
    fn default_backoff_multiplier() -> f64 {
        2.0
    }
}

/// External data value that can be used in Roto filters
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExternalDataValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<ExternalDataValue>),
    Object(HashMap<String, ExternalDataValue>),
    Null,
}

/// Cached external data entry
#[derive(Clone, Debug)]
pub struct CachedData {
    pub value: ExternalDataValue,
    pub fetched_at: Instant,
    pub ttl: Duration,
}

impl CachedData {
    pub fn new(value: ExternalDataValue, ttl: Duration) -> Self {
        Self {
            value,
            fetched_at: Instant::now(),
            ttl,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        self.fetched_at.elapsed() > self.ttl
    }
}

/// External data manager
pub struct ExternalDataManager {
    sources: HashMap<String, ExternalDataSource>,
    cache: Arc<RwLock<HashMap<String, CachedData>>>,
    refresh_tx: mpsc::UnboundedSender<String>,
}

impl ExternalDataManager {
    pub fn new() -> Self {
        let (refresh_tx, refresh_rx) = mpsc::unbounded_channel();
        let cache = Arc::new(RwLock::new(HashMap::new()));
        
        // Start background refresh task
        let cache_clone = cache.clone();
        tokio::spawn(Self::refresh_task(refresh_rx, cache_clone));
        
        Self {
            sources: HashMap::new(),
            cache,
            refresh_tx,
        }
    }
    
    /// Add an external data source
    pub fn add_source(&mut self, source: ExternalDataSource) {
        let source_id = source.id.clone();
        self.sources.insert(source_id.clone(), source);
        
        // Trigger initial fetch
        if let Err(e) = self.refresh_tx.send(source_id) {
            error!("Failed to trigger initial fetch for external data source: {}", e);
        }
    }
    
    /// Remove an external data source
    pub fn remove_source(&mut self, source_id: &str) {
        self.sources.remove(source_id);
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(source_id);
        }
    }
    
    /// Get data from an external source
    pub async fn get_data(&self, source_id: &str) -> Option<ExternalDataValue> {
        // Check cache first
        if let Ok(cache) = self.cache.read() {
            if let Some(cached) = cache.get(source_id) {
                if !cached.is_expired() {
                    debug!("Returning cached data for source: {}", source_id);
                    return Some(cached.value.clone());
                }
            }
        }
        
        // Cache miss or expired, trigger refresh
        if let Err(e) = self.refresh_tx.send(source_id.to_string()) {
            error!("Failed to trigger refresh for external data source {}: {}", source_id, e);
        }
        
        // Return stale data if available
        if let Ok(cache) = self.cache.read() {
            if let Some(cached) = cache.get(source_id) {
                warn!("Returning stale data for source: {}", source_id);
                return Some(cached.value.clone());
            }
        }
        
        None
    }
    
    /// Background task for refreshing external data
    async fn refresh_task(
        mut refresh_rx: mpsc::UnboundedReceiver<String>,
        cache: Arc<RwLock<HashMap<String, CachedData>>>,
    ) {
        while let Some(source_id) = refresh_rx.recv().await {
            debug!("Refreshing external data source: {}", source_id);
            
            // TODO: Implement actual data fetching based on source type
            // For now, this is a placeholder
            let placeholder_data = ExternalDataValue::Object({
                let mut map = HashMap::new();
                map.insert("source_id".to_string(), ExternalDataValue::String(source_id.clone()));
                map.insert("timestamp".to_string(), ExternalDataValue::Number(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as f64
                ));
                map.insert("status".to_string(), ExternalDataValue::String("active".to_string()));
                map
            });
            
            let cached_data = CachedData::new(
                placeholder_data,
                Duration::from_secs(300), // 5 minutes TTL
            );
            
            if let Ok(mut cache) = cache.write() {
                cache.insert(source_id.clone(), cached_data);
                debug!("Updated cache for external data source: {}", source_id);
            } else {
                error!("Failed to update cache for external data source: {}", source_id);
            }
        }
    }
    
    /// Start automatic refresh for all sources
    pub fn start_auto_refresh(&self) {
        for (source_id, source) in &self.sources {
            if source.auto_refresh {
                let source_id = source_id.clone();
                let refresh_tx = self.refresh_tx.clone();
                let refresh_interval = Duration::from_secs(source.refresh_interval_secs);
                
                tokio::spawn(async move {
                    let mut interval = interval(refresh_interval);
                    loop {
                        interval.tick().await;
                        if let Err(e) = refresh_tx.send(source_id.clone()) {
                            error!("Failed to send refresh signal for {}: {}", source_id, e);
                            break;
                        }
                    }
                });
            }
        }
    }
}

impl Default for ExternalDataManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for accessing external data in Roto filters
pub trait ExternalDataAccess {
    /// Get external data by source ID
    fn get_external_data(&self, source_id: &str) -> Option<ExternalDataValue>;
    
    /// Check if external data source exists
    fn has_external_data(&self, source_id: &str) -> bool;
    
    /// Get external data as string
    fn get_external_string(&self, source_id: &str) -> Option<String> {
        match self.get_external_data(source_id)? {
            ExternalDataValue::String(s) => Some(s),
            ExternalDataValue::Number(n) => Some(n.to_string()),
            ExternalDataValue::Boolean(b) => Some(b.to_string()),
            _ => None,
        }
    }
    
    /// Get external data as number
    fn get_external_number(&self, source_id: &str) -> Option<f64> {
        match self.get_external_data(source_id)? {
            ExternalDataValue::Number(n) => Some(n),
            ExternalDataValue::String(s) => s.parse().ok(),
            _ => None,
        }
    }
    
    /// Get external data as boolean
    fn get_external_boolean(&self, source_id: &str) -> Option<bool> {
        match self.get_external_data(source_id)? {
            ExternalDataValue::Boolean(b) => Some(b),
            ExternalDataValue::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            },
            ExternalDataValue::Number(n) => Some(n != 0.0),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_data_source_deserialization() {
        let toml = r#"
        id = "test-source"
        type = "http"
        url = "https://api.example.com/data"
        method = "GET"
        timeout_secs = 60
        refresh_interval_secs = 600
        cache_ttl_secs = 1200
        auto_refresh = true
        
        [headers]
        "Content-Type" = "application/json"
        "User-Agent" = "Rotonda/1.0"
        
        [auth]
        type = "bearer"
        token = "secret-token"
        "#;
        
        let source: ExternalDataSource = toml::from_str(toml).unwrap();
        
        assert_eq!(source.id, "test-source");
        assert_eq!(source.refresh_interval_secs, 600);
        assert_eq!(source.cache_ttl_secs, 1200);
        assert!(source.auto_refresh);
        
        if let ExternalDataSourceType::Http(http_source) = source.source_type {
            assert_eq!(http_source.url.as_str(), "https://api.example.com/data");
            assert_eq!(http_source.method, "GET");
            assert_eq!(http_source.timeout_secs, 60);
            assert_eq!(http_source.headers.get("Content-Type"), Some(&"application/json".to_string()));
            assert!(http_source.auth.is_some());
        } else {
            panic!("Expected HTTP data source");
        }
    }

    #[test]
    fn test_cached_data_expiration() {
        let data = ExternalDataValue::String("test".to_string());
        let ttl = Duration::from_millis(100);
        let cached = CachedData::new(data, ttl);
        
        assert!(!cached.is_expired());
        
        std::thread::sleep(Duration::from_millis(150));
        assert!(cached.is_expired());
    }

    #[test]
    fn test_external_data_value_serialization() {
        let value = ExternalDataValue::Object({
            let mut map = HashMap::new();
            map.insert("string".to_string(), ExternalDataValue::String("test".to_string()));
            map.insert("number".to_string(), ExternalDataValue::Number(42.0));
            map.insert("boolean".to_string(), ExternalDataValue::Boolean(true));
            map
        });
        
        let json = serde_json::to_string(&value).unwrap();
        let deserialized: ExternalDataValue = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            ExternalDataValue::Object(map) => {
                assert_eq!(map.get("string"), Some(&ExternalDataValue::String("test".to_string())));
                assert_eq!(map.get("number"), Some(&ExternalDataValue::Number(42.0)));
                assert_eq!(map.get("boolean"), Some(&ExternalDataValue::Boolean(true)));
            }
            _ => panic!("Expected object"),
        }
    }
}