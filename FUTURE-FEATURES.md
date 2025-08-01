# Rotonda Future Features Implementation

This document describes the implementation of the future features mentioned in the original README:

> Future versions of Rotonda will support an on-disk database, using external datasets in filters, reading routes from Kafka streams, and more.

## 🎯 Implemented Features

### 1. On-Disk Database Support

Rotonda now supports persistent storage with multiple storage backends:

#### Storage Types

- **Memory**: Traditional in-memory storage (default)
- **Disk**: Full on-disk persistence with optional memory cache
- **Hybrid**: Intelligent tiering between memory and disk

#### Configuration Example

```toml
[units.rib.storage]
type = "hybrid"
placement_strategy = "recent_in_memory"
memory_threshold = 500000
auto_migration = true

[units.rib.storage.disk]
path = "/var/lib/rotonda/rib-data"
max_size_bytes = 10737418240  # 10GB
compression = true
sync_mode = "normal"
cache_size = 50000
compaction_interval_secs = 3600
```

#### Features

- **Compression**: Reduce storage footprint
- **Configurable sync modes**: Balance between performance and durability
- **Background compaction**: Automatic space reclamation
- **Cache management**: Intelligent memory caching of hot data
- **Data migration**: Automatic movement between storage tiers

### 2. Kafka Stream Input

Rotonda can now consume BGP/BMP data from Kafka streams:

#### Configuration Example

```toml
[units.kafka-bgp-updates]
type = "kafka-in"
brokers = ["kafka1.example.com:9092", "kafka2.example.com:9092"]
topic = "bgp-updates"
group_id = "rotonda-consumer"
format = "json"

[units.kafka-bgp-updates.consumer_config]
auto_offset_reset = "latest"
enable_auto_commit = true
session_timeout_ms = 30000

[units.kafka-bgp-updates.retry_config]
max_retries = 5
initial_delay_ms = 1000
backoff_multiplier = 2.0
```

#### Features

- **Multiple message formats**: JSON, MRT, BGP UPDATE, custom parsers
- **Consumer group support**: Scalable consumption with multiple instances
- **Retry logic**: Exponential backoff with configurable limits
- **Health monitoring**: Built-in health checks and metrics
- **Offset management**: Automatic or manual commit strategies

### 3. External Datasets in Filters

Roto filters can now access external data sources for enhanced routing decisions:

#### Supported Data Sources

- **HTTP/HTTPS APIs**: RESTful services with authentication
- **Files**: JSON, YAML, TOML, CSV formats with file watching
- **Databases**: PostgreSQL, MySQL with connection pooling
- **Redis**: Key-value and hash operations
- **Other RIBs**: Cross-RIB data queries

#### Configuration Example

```toml
[[external_data_sources]]
id = "rpki-validator"
type = "http"
url = "https://rpki-validator.example.com/api/v1/validity"
refresh_interval_secs = 300
cache_ttl_secs = 600
auto_refresh = true

[external_data_sources.auth]
type = "bearer"
token = "your-api-token-here"
```

#### Roto Filter Usage

```roto
// External data sources
external rpki_validator: ExternalData = "rpki-validator";
external customer_prefixes: ExternalData = "customer-prefixes";

filter enhanced_route_filter for route: Route {
    let prefix = route.prefix();
    let origin_asn = route.origin_asn();
    
    // Enhanced RPKI validation using external validator
    if let Some(rpki_data) = rpki_validator.get_object(format!("{}:{}", prefix, origin_asn)) {
        if let Some(validity) = rpki_data.get_string("validity") {
            match validity {
                "valid" => {
                    route.set_rpki_status(RpkiStatus::Valid);
                    route.add_community(Community::new(65000, 1));
                }
                "invalid" => {
                    route.set_rpki_status(RpkiStatus::Invalid);
                    return reject;
                }
                _ => {}
            }
        }
    }
    
    return accept;
}
```

## 🏗️ Architecture Overview

### Storage Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Memory Tier   │    │   Disk Tier     │    │  External Data  │
│                 │    │                 │    │                 │
│ • Hot routes    │◄──►│ • Cold routes   │    │ • HTTP APIs     │
│ • Fast access   │    │ • Compressed    │    │ • Databases     │
│ • LRU eviction  │    │ • Persistent    │    │ • Files         │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Data Flow

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Kafka     │    │   Filters   │    │     RIB     │    │   Outputs   │
│             │    │             │    │             │    │             │
│ • BGP msgs  │───►│ • External  │───►│ • Storage   │───►│ • MQTT      │
│ • JSON/MRT  │    │   data      │    │ • Queries   │    │ • Files     │
│ • Reliable  │    │ • Enhanced  │    │ • APIs      │    │ • Database  │
└─────────────┘    └─────────────┘    └─────────────┘    └─────────────┘
```

## 🚀 Getting Started

### 1. Basic Setup with Docker Compose

```bash
# Start with basic services
docker-compose up -d rotonda

# Start with Kafka support
docker-compose --profile kafka up -d

# Start full monitoring stack
docker-compose --profile full up -d
```

### 2. Configuration

Copy the advanced configuration:

```bash
cp etc/rotonda-advanced.conf etc/rotonda.conf
cp etc/filters-advanced.roto filters.roto
```

### 3. External Data Setup

Create external data directories:

```bash
mkdir -p /etc/rotonda/data
# Add your external data files (JSON, YAML, etc.)
```

### 4. Database Setup (Optional)

For PostgreSQL external data:

```sql
CREATE DATABASE routing_db;
CREATE TABLE customer_prefixes (
    prefix CIDR,
    customer_id INTEGER,
    tier VARCHAR(10),
    active BOOLEAN DEFAULT true
);
```

## 📊 Monitoring and Metrics

### New Metrics

- **Storage metrics**: Disk usage, cache hit rates, compaction stats
- **Kafka metrics**: Consumer lag, message rates, error counts
- **External data metrics**: Fetch times, cache performance, error rates

### Prometheus Queries

```promql
# Storage cache hit rate
rotonda_storage_cache_hit_rate

# Kafka consumer lag
rotonda_kafka_consumer_lag

# External data fetch duration
rotonda_external_data_fetch_duration_seconds
```

## 🔧 Advanced Configuration

### Storage Tuning

```toml
[units.rib.storage.disk]
# Optimize for write-heavy workloads
sync_mode = "none"
compaction_interval_secs = 7200

# Optimize for read-heavy workloads
cache_size = 100000
compression = false
```

### Kafka Tuning

```toml
[units.kafka-in.consumer_config]
# High throughput settings
fetch_min_bytes = 1048576  # 1MB
fetch_max_wait_ms = 100

# Low latency settings
fetch_min_bytes = 1
fetch_max_wait_ms = 10
```

### External Data Optimization

```toml
[[external_data_sources]]
# Frequent updates
refresh_interval_secs = 60
cache_ttl_secs = 120

# Infrequent updates
refresh_interval_secs = 3600
cache_ttl_secs = 7200
```

## 🧪 Testing

### Unit Tests

```bash
# Test storage backends
cargo test storage::tests

# Test Kafka integration
cargo test kafka_in::tests

# Test external data
cargo test external_data::tests
```

### Integration Tests

```bash
# Start test environment
docker-compose -f docker-compose.test.yml up -d

# Run integration tests
cargo test --test integration
```

## 🔮 Future Enhancements

### Planned Features

1. **Advanced Storage**
   - Distributed storage across multiple nodes
   - Automatic sharding and replication
   - Time-series optimizations

2. **Enhanced Kafka Integration**
   - Schema registry support
   - Avro/Protobuf message formats
   - Kafka Streams integration

3. **External Data Improvements**
   - GraphQL API support
   - Real-time data streaming
   - Machine learning model integration

4. **Performance Optimizations**
   - SIMD-accelerated processing
   - GPU-based route analysis
   - Advanced caching strategies

### Roadmap

- **Q1 2024**: Distributed storage, Schema registry
- **Q2 2024**: GraphQL support, ML integration
- **Q3 2024**: GPU acceleration, Advanced analytics
- **Q4 2024**: Cloud-native deployment, Auto-scaling

## 📚 Documentation

- [Storage Configuration Guide](docs/storage.md)
- [Kafka Integration Guide](docs/kafka.md)
- [External Data Sources Guide](docs/external-data.md)
- [Roto Filter Examples](docs/roto-examples.md)
- [Performance Tuning Guide](docs/performance.md)

## 🤝 Contributing

We welcome contributions! Please see:

- [Contributing Guidelines](CONTRIBUTING.md)
- [Development Setup](docs/development.md)
- [Architecture Overview](docs/architecture.md)

## 📄 License

This project is licensed under the Mozilla Public License 2.0 - see the [LICENSE](LICENSE) file for details.