use std::path::PathBuf;
use serde::Deserialize;
use rotonda_store::rib::config::{MemoryOnlyConfig, Config as RibConfig};

/// Storage configuration for RIB units
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum StorageConfig {
    /// In-memory storage only (default)
    #[serde(rename = "memory")]
    Memory(MemoryOnlyConfig),
    
    /// On-disk storage with optional memory cache
    #[serde(rename = "disk")]
    Disk(DiskStorageConfig),
    
    /// Hybrid storage: memory + disk persistence
    #[serde(rename = "hybrid")]
    Hybrid(HybridStorageConfig),
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig::Memory(MemoryOnlyConfig::default())
    }
}

/// Configuration for on-disk storage
#[derive(Clone, Debug, Deserialize)]
pub struct DiskStorageConfig {
    /// Path to the database directory
    pub path: PathBuf,
    
    /// Maximum size of the database in bytes (optional)
    pub max_size_bytes: Option<u64>,
    
    /// Enable compression for stored data
    #[serde(default = "DiskStorageConfig::default_compression")]
    pub compression: bool,
    
    /// Sync mode for writes
    #[serde(default = "DiskStorageConfig::default_sync_mode")]
    pub sync_mode: SyncMode,
    
    /// Cache size in memory (in number of entries)
    #[serde(default = "DiskStorageConfig::default_cache_size")]
    pub cache_size: usize,
    
    /// Background compaction interval in seconds
    #[serde(default = "DiskStorageConfig::default_compaction_interval")]
    pub compaction_interval_secs: u64,
}

impl DiskStorageConfig {
    fn default_compression() -> bool {
        true
    }
    
    fn default_sync_mode() -> SyncMode {
        SyncMode::Normal
    }
    
    fn default_cache_size() -> usize {
        10_000
    }
    
    fn default_compaction_interval() -> u64 {
        3600 // 1 hour
    }
}

/// Configuration for hybrid storage (memory + disk)
#[derive(Clone, Debug, Deserialize)]
pub struct HybridStorageConfig {
    /// Disk storage configuration
    pub disk: DiskStorageConfig,
    
    /// Memory storage configuration
    pub memory: MemoryOnlyConfig,
    
    /// Strategy for data placement
    #[serde(default = "HybridStorageConfig::default_placement_strategy")]
    pub placement_strategy: PlacementStrategy,
    
    /// Threshold for moving data from memory to disk (number of routes)
    #[serde(default = "HybridStorageConfig::default_memory_threshold")]
    pub memory_threshold: usize,
    
    /// Enable automatic data migration between tiers
    #[serde(default = "HybridStorageConfig::default_auto_migration")]
    pub auto_migration: bool,
}

impl HybridStorageConfig {
    fn default_placement_strategy() -> PlacementStrategy {
        PlacementStrategy::RecentInMemory
    }
    
    fn default_memory_threshold() -> usize {
        100_000
    }
    
    fn default_auto_migration() -> bool {
        true
    }
}

/// Synchronization mode for disk writes
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncMode {
    /// No explicit sync (fastest, least durable)
    None,
    /// Normal sync (balanced)
    Normal,
    /// Full sync (slowest, most durable)
    Full,
}

/// Strategy for placing data in hybrid storage
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementStrategy {
    /// Keep recent routes in memory, older ones on disk
    RecentInMemory,
    /// Keep frequently accessed routes in memory
    FrequentInMemory,
    /// Keep routes from specific peers in memory
    PeerBasedMemory,
    /// Custom placement based on route attributes
    Custom,
}

impl StorageConfig {
    /// Convert to rotonda-store RibConfig
    pub fn to_rib_config(&self) -> RibConfig {
        match self {
            StorageConfig::Memory(config) => RibConfig::MemoryOnly(config.clone()),
            StorageConfig::Disk(_config) => {
                // For now, fall back to memory-only until we implement disk storage
                // TODO: Implement actual disk storage backend
                RibConfig::MemoryOnly(MemoryOnlyConfig::default())
            },
            StorageConfig::Hybrid(_config) => {
                // For now, fall back to memory-only until we implement hybrid storage
                // TODO: Implement actual hybrid storage backend
                RibConfig::MemoryOnly(MemoryOnlyConfig::default())
            },
        }
    }
    
    /// Check if this storage configuration supports persistence
    pub fn is_persistent(&self) -> bool {
        matches!(self, StorageConfig::Disk(_) | StorageConfig::Hybrid(_))
    }
    
    /// Get the storage type name for logging/metrics
    pub fn storage_type(&self) -> &'static str {
        match self {
            StorageConfig::Memory(_) => "memory",
            StorageConfig::Disk(_) => "disk",
            StorageConfig::Hybrid(_) => "hybrid",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_default_storage_config() {
        let config = StorageConfig::default();
        assert!(matches!(config, StorageConfig::Memory(_)));
        assert!(!config.is_persistent());
        assert_eq!(config.storage_type(), "memory");
    }

    #[test]
    fn test_disk_storage_config_deserialization() {
        let toml = r#"
        type = "disk"
        path = "/var/lib/rotonda"
        max_size_bytes = 1073741824
        compression = true
        sync_mode = "normal"
        cache_size = 50000
        compaction_interval_secs = 7200
        "#;
        
        let config: StorageConfig = toml::from_str(toml).unwrap();
        
        if let StorageConfig::Disk(disk_config) = config {
            assert_eq!(disk_config.path, Path::new("/var/lib/rotonda"));
            assert_eq!(disk_config.max_size_bytes, Some(1073741824));
            assert!(disk_config.compression);
            assert!(matches!(disk_config.sync_mode, SyncMode::Normal));
            assert_eq!(disk_config.cache_size, 50000);
            assert_eq!(disk_config.compaction_interval_secs, 7200);
        } else {
            panic!("Expected DiskStorageConfig");
        }
    }

    #[test]
    fn test_hybrid_storage_config_deserialization() {
        let toml = r#"
        type = "hybrid"
        placement_strategy = "frequent_in_memory"
        memory_threshold = 200000
        auto_migration = false
        
        [disk]
        path = "/var/lib/rotonda"
        compression = false
        
        [memory]
        # memory config options would go here
        "#;
        
        let config: StorageConfig = toml::from_str(toml).unwrap();
        
        if let StorageConfig::Hybrid(hybrid_config) = config {
            assert!(matches!(hybrid_config.placement_strategy, PlacementStrategy::FrequentInMemory));
            assert_eq!(hybrid_config.memory_threshold, 200000);
            assert!(!hybrid_config.auto_migration);
            assert_eq!(hybrid_config.disk.path, Path::new("/var/lib/rotonda"));
            assert!(!hybrid_config.disk.compression);
        } else {
            panic!("Expected HybridStorageConfig");
        }
    }
}