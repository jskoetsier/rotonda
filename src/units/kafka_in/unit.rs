use crate::{
    comms::{Gate, GateStatus, Terminated},
    manager::{Component, WaitPoint},
    payload::{Payload, RouteContext, UpstreamStatus},
    units::Unit,
};
use async_trait::async_trait;
use chrono::Utc;
use log::{debug, error, info, warn};
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::mpsc,
    time::{interval, sleep},
};

/// Kafka consumer configuration
#[derive(Clone, Debug, Deserialize)]
pub struct KafkaIn {
    /// Kafka broker addresses
    pub brokers: Vec<String>,
    
    /// Topic to consume from
    pub topic: String,
    
    /// Consumer group ID
    pub group_id: String,
    
    /// Message format
    #[serde(default = "KafkaIn::default_format")]
    pub format: MessageFormat,
    
    /// Consumer configuration options
    #[serde(default)]
    pub consumer_config: KafkaConsumerConfig,
    
    /// Retry configuration
    #[serde(default)]
    pub retry_config: RetryConfig,
    
    /// Optional filter for messages
    pub message_filter: Option<String>,
}

impl KafkaIn {
    fn default_format() -> MessageFormat {
        MessageFormat::Json
    }

    pub async fn run(
        self,
        component: Component,
        gate: Gate,
        waitpoint: WaitPoint,
    ) -> Result<(), Terminated> {
        KafkaInRunner::new(self, component, gate)
            .run(waitpoint)
            .await
    }
}

/// Message format for Kafka messages
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageFormat {
    /// JSON format
    Json,
    /// MRT format (binary)
    Mrt,
    /// BGP UPDATE messages
    BgpUpdate,
    /// Custom format with parser
    Custom(String),
}

/// Kafka consumer configuration
#[derive(Clone, Debug, Deserialize)]
pub struct KafkaConsumerConfig {
    /// Auto offset reset strategy
    #[serde(default = "KafkaConsumerConfig::default_auto_offset_reset")]
    pub auto_offset_reset: String,
    
    /// Enable auto commit
    #[serde(default = "KafkaConsumerConfig::default_enable_auto_commit")]
    pub enable_auto_commit: bool,
    
    /// Auto commit interval in milliseconds
    #[serde(default = "KafkaConsumerConfig::default_auto_commit_interval_ms")]
    pub auto_commit_interval_ms: u64,
    
    /// Session timeout in milliseconds
    #[serde(default = "KafkaConsumerConfig::default_session_timeout_ms")]
    pub session_timeout_ms: u64,
    
    /// Fetch minimum bytes
    #[serde(default = "KafkaConsumerConfig::default_fetch_min_bytes")]
    pub fetch_min_bytes: i32,
    
    /// Fetch maximum wait time in milliseconds
    #[serde(default = "KafkaConsumerConfig::default_fetch_max_wait_ms")]
    pub fetch_max_wait_ms: i32,
    
    /// Additional consumer properties
    #[serde(default)]
    pub additional_properties: HashMap<String, String>,
}

impl Default for KafkaConsumerConfig {
    fn default() -> Self {
        Self {
            auto_offset_reset: Self::default_auto_offset_reset(),
            enable_auto_commit: Self::default_enable_auto_commit(),
            auto_commit_interval_ms: Self::default_auto_commit_interval_ms(),
            session_timeout_ms: Self::default_session_timeout_ms(),
            fetch_min_bytes: Self::default_fetch_min_bytes(),
            fetch_max_wait_ms: Self::default_fetch_max_wait_ms(),
            additional_properties: HashMap::new(),
        }
    }
}

impl KafkaConsumerConfig {
    fn default_auto_offset_reset() -> String {
        "latest".to_string()
    }
    
    fn default_enable_auto_commit() -> bool {
        true
    }
    
    fn default_auto_commit_interval_ms() -> u64 {
        5000
    }
    
    fn default_session_timeout_ms() -> u64 {
        30000
    }
    
    fn default_fetch_min_bytes() -> i32 {
        1
    }
    
    fn default_fetch_max_wait_ms() -> i32 {
        500
    }
}

/// Retry configuration for Kafka operations
#[derive(Clone, Debug, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries
    #[serde(default = "RetryConfig::default_max_retries")]
    pub max_retries: u32,
    
    /// Initial retry delay in milliseconds
    #[serde(default = "RetryConfig::default_initial_delay_ms")]
    pub initial_delay_ms: u64,
    
    /// Maximum retry delay in milliseconds
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
        5
    }
    
    fn default_initial_delay_ms() -> u64 {
        1000
    }
    
    fn default_max_delay_ms() -> u64 {
        30000
    }
    
    fn default_backoff_multiplier() -> f64 {
        2.0
    }
}

/// Kafka input unit runner
pub struct KafkaInRunner {
    config: KafkaIn,
    component: Component,
    gate: Arc<Gate>,
}

impl KafkaInRunner {
    fn new(config: KafkaIn, component: Component, gate: Gate) -> Self {
        Self {
            config,
            component,
            gate: Arc::new(gate),
        }
    }

    async fn run(self, mut waitpoint: WaitPoint) -> Result<(), Terminated> {
        info!(
            "Starting Kafka consumer for topic '{}' from brokers: {:?}",
            self.config.topic, self.config.brokers
        );

        // Wait for other components to be ready
        self.gate.process_until(waitpoint.ready()).await?;
        waitpoint.running().await;

        // Start the Kafka consumer task
        let consumer_task = self.start_consumer_task();
        
        // Main event loop
        loop {
            tokio::select! {
                // Handle gate events
                gate_result = self.gate.process() => {
                    match gate_result {
                        Ok(status) => {
                            match status {
                                GateStatus::Reconfiguring { new_config } => {
                                    if let Unit::KafkaIn(new_kafka_config) = new_config {
                                        info!("Reconfiguring Kafka consumer");
                                        // TODO: Implement reconfiguration
                                        // For now, we'll just log the change
                                        warn!("Kafka reconfiguration not yet implemented");
                                    }
                                }
                                GateStatus::ReportLinks { report } => {
                                    report.set_graph_status(self.gate.metrics());
                                }
                                _ => {}
                            }
                        }
                        Err(Terminated) => {
                            info!("Kafka consumer terminated");
                            return Err(Terminated);
                        }
                    }
                }
                
                // Handle consumer task completion (shouldn't happen in normal operation)
                _ = &mut consumer_task => {
                    error!("Kafka consumer task completed unexpectedly");
                    return Err(Terminated);
                }
            }
        }
    }

    fn start_consumer_task(&self) -> tokio::task::JoinHandle<()> {
        let config = self.config.clone();
        let gate = self.gate.clone();
        
        tokio::spawn(async move {
            let mut retry_count = 0;
            let mut delay = Duration::from_millis(config.retry_config.initial_delay_ms);
            
            loop {
                match Self::run_consumer(&config, &gate).await {
                    Ok(()) => {
                        info!("Kafka consumer completed successfully");
                        break;
                    }
                    Err(e) => {
                        error!("Kafka consumer error: {}", e);
                        
                        if retry_count >= config.retry_config.max_retries {
                            error!("Max retries exceeded, stopping Kafka consumer");
                            break;
                        }
                        
                        retry_count += 1;
                        warn!(
                            "Retrying Kafka consumer in {}ms (attempt {}/{})",
                            delay.as_millis(),
                            retry_count,
                            config.retry_config.max_retries
                        );
                        
                        sleep(delay).await;
                        
                        // Exponential backoff
                        delay = Duration::from_millis(std::cmp::min(
                            (delay.as_millis() as f64 * config.retry_config.backoff_multiplier) as u64,
                            config.retry_config.max_delay_ms,
                        ));
                    }
                }
            }
        })
    }

    async fn run_consumer(config: &KafkaIn, gate: &Gate) -> Result<(), String> {
        // TODO: Implement actual Kafka consumer using rdkafka or similar
        // For now, this is a placeholder implementation
        
        info!("Starting Kafka consumer (placeholder implementation)");
        
        // Simulate consuming messages
        let mut interval = interval(Duration::from_secs(5));
        let mut message_count = 0;
        
        loop {
            interval.tick().await;
            
            // Simulate receiving a message
            message_count += 1;
            debug!("Simulated Kafka message #{}", message_count);
            
            // Create a placeholder payload
            // In a real implementation, this would parse the Kafka message
            // and convert it to the appropriate Rotonda payload format
            let payload = Self::create_placeholder_payload(message_count);
            
            // Send the payload downstream
            gate.update_data(crate::payload::Update::Single(payload)).await;
            
            // For demonstration, stop after 10 messages
            if message_count >= 10 {
                info!("Stopping placeholder Kafka consumer after {} messages", message_count);
                break;
            }
        }
        
        Ok(())
    }
    
    fn create_placeholder_payload(message_id: u32) -> Payload {
        use crate::payload::{RotondaRoute, Provenance};
        use inetnum::{addr::Prefix, asn::Asn};
        use routecore::bgp::types::AfiSafiType;
        use std::str::FromStr;
        
        // Create a placeholder route
        let prefix = Prefix::from_str(&format!("192.0.2.{}/24", message_id % 256))
            .unwrap_or_else(|_| Prefix::from_str("192.0.2.0/24").unwrap());
        
        let route = RotondaRoute::new_with_local_pref(
            prefix,
            AfiSafiType::Ipv4Unicast,
            Some(100),
        );
        
        let provenance = Provenance::new(
            message_id, // ingress_id
            Some(Asn::from_u32(65000 + message_id)), // remote_asn
            format!("kafka-message-{}", message_id), // connection_id
        );
        
        let context = RouteContext::for_kafka_message(
            crate::payload::RouteStatus::InConvergence,
            provenance,
        );
        
        Payload::new(route, context, None)
    }
}

// Extension trait for RouteContext to support Kafka messages
trait RouteContextExt {
    fn for_kafka_message(
        status: crate::payload::RouteStatus,
        provenance: crate::payload::Provenance,
    ) -> Self;
}

impl RouteContextExt for RouteContext {
    fn for_kafka_message(
        status: crate::payload::RouteStatus,
        provenance: crate::payload::Provenance,
    ) -> Self {
        // For now, use the Fresh context type
        // In a real implementation, we might want a dedicated Kafka context type
        RouteContext::Fresh(crate::payload::FreshRouteContext::new(
            status,
            provenance,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kafka_config_deserialization() {
        let toml = r#"
        brokers = ["localhost:9092", "localhost:9093"]
        topic = "bgp-updates"
        group_id = "rotonda-consumer"
        format = "json"
        
        [consumer_config]
        auto_offset_reset = "earliest"
        enable_auto_commit = false
        session_timeout_ms = 60000
        
        [retry_config]
        max_retries = 10
        initial_delay_ms = 2000
        "#;
        
        let config: KafkaIn = toml::from_str(toml).unwrap();
        
        assert_eq!(config.brokers, vec!["localhost:9092", "localhost:9093"]);
        assert_eq!(config.topic, "bgp-updates");
        assert_eq!(config.group_id, "rotonda-consumer");
        assert!(matches!(config.format, MessageFormat::Json));
        assert_eq!(config.consumer_config.auto_offset_reset, "earliest");
        assert!(!config.consumer_config.enable_auto_commit);
        assert_eq!(config.consumer_config.session_timeout_ms, 60000);
        assert_eq!(config.retry_config.max_retries, 10);
        assert_eq!(config.retry_config.initial_delay_ms, 2000);
    }

    #[test]
    fn test_default_kafka_config() {
        let config = KafkaConsumerConfig::default();
        assert_eq!(config.auto_offset_reset, "latest");
        assert!(config.enable_auto_commit);
        assert_eq!(config.auto_commit_interval_ms, 5000);
    }

    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
        assert_eq!(config.backoff_multiplier, 2.0);
    }
}