# Rotonda Docker Compose Setup

This directory contains a complete Docker Compose setup for running Rotonda with optional supporting services for monitoring and testing.

## Quick Start

### Basic Rotonda Only
```bash
# Build and start Rotonda
docker-compose up -d rotonda

# View logs
docker-compose logs -f rotonda

# Check status
docker-compose ps
```

### With MQTT Broker (for testing)
```bash
# Start Rotonda with MQTT broker
docker-compose --profile mqtt up -d

# Access MQTT broker at localhost:1883
# WebSocket interface at localhost:9001
```

### Full Monitoring Stack
```bash
# Start everything (Rotonda + MQTT + Prometheus + Grafana)
docker-compose --profile full up -d

# Access services:
# - Rotonda API: http://localhost:8080
# - Prometheus: http://localhost:9090
# - Grafana: http://localhost:3000 (admin/admin)
# - MQTT: localhost:1883
```

## Services Overview

### Core Services

#### Rotonda
- **Ports**: 8080 (HTTP API), 11019 (BMP), 11179 (BGP)
- **Config**: Mounted from `./etc/rotonda.conf`
- **Health Check**: HTTP endpoint monitoring
- **Logs**: Persistent volume for log storage

### Optional Services (Profiles)

#### MQTT Broker (`--profile mqtt`)
- **Image**: Eclipse Mosquitto 2.0
- **Ports**: 1883 (MQTT), 9001 (WebSocket)
- **Config**: Custom configuration for development
- **Use Case**: Testing MQTT output targets

#### Prometheus (`--profile monitoring`)
- **Image**: Prometheus latest
- **Port**: 9090
- **Config**: Pre-configured to scrape Rotonda metrics
- **Retention**: 200 hours of metrics data

#### Grafana (`--profile full`)
- **Image**: Grafana latest
- **Port**: 3000
- **Credentials**: admin/admin
- **Features**: Pre-configured Prometheus datasource

## Configuration

### Rotonda Configuration
Edit `./etc/rotonda.conf` to customize Rotonda behavior:

```toml
# Example MQTT target configuration
[targets.mqtt]
type = "mqtt-out"
sources = ["rib"]
destination = "mqtt-broker"  # Use service name
client_id = "rotonda"
```

### Custom Build Arguments
Modify `docker-compose.yml` to customize the build:

```yaml
services:
  rotonda:
    build:
      args:
        MODE: build
        BASE_IMG: alpine:3.18
        CARGO_ARGS: "--no-default-features"
```

## Network Architecture

All services run on a custom bridge network (`rotonda-net`) with subnet `172.20.0.0/16`:

- **rotonda**: Main BGP/BMP processing engine
- **mqtt-broker**: Message broker for route updates
- **prometheus**: Metrics collection and storage
- **grafana**: Metrics visualization and dashboards

## Data Persistence

### Volumes
- `rotonda-logs`: Application logs
- `mqtt-data`: MQTT broker persistence
- `prometheus-data`: Metrics storage
- `grafana-data`: Dashboard and user data

### Backup
```bash
# Backup all volumes
docker run --rm -v rotonda-logs:/data -v $(pwd):/backup alpine tar czf /backup/rotonda-backup.tar.gz /data

# Restore
docker run --rm -v rotonda-logs:/data -v $(pwd):/backup alpine tar xzf /backup/rotonda-backup.tar.gz -C /
```

## Development Workflow

### Building and Testing
```bash
# Rebuild after code changes
docker-compose build rotonda

# Run with development logging
docker-compose up rotonda
```

### Debugging
```bash
# Access container shell
docker-compose exec rotonda sh

# View real-time logs
docker-compose logs -f --tail=100 rotonda

# Check metrics
curl http://localhost:8080/metrics
```

### Configuration Reload
```bash
# Send SIGHUP to reload configuration
docker-compose kill -s HUP rotonda
```

## Production Considerations

### Security
- Change default Grafana password
- Configure MQTT authentication
- Use secrets management for sensitive data
- Enable TLS for external connections

### Scaling
- Use external databases for persistence
- Configure load balancing for multiple instances
- Monitor resource usage and adjust limits

### Monitoring
- Set up alerting rules in Prometheus
- Configure log aggregation
- Monitor container health and resource usage

## Troubleshooting

### Common Issues

**Port Conflicts**
```bash
# Check port usage
netstat -tulpn | grep :8080

# Use different ports
docker-compose up -d --scale rotonda=0
docker-compose run -p 8081:8080 rotonda
```

**Build Failures**
```bash
# Clean build
docker-compose build --no-cache rotonda

# Check build logs
docker-compose build rotonda 2>&1 | tee build.log
```

**Configuration Issues**
```bash
# Validate configuration
docker-compose config

# Check mounted files
docker-compose exec rotonda ls -la /etc/rotonda/
```

### Logs and Debugging
```bash
# All service logs
docker-compose logs

# Specific service
docker-compose logs rotonda

# Follow logs
docker-compose logs -f --tail=50 rotonda
```

## API Endpoints

### Rotonda HTTP API
- `GET /metrics` - Prometheus metrics
- `GET /prefixes/{prefix}` - Query specific prefix
- `GET /prefixes/{vrib_id}/{prefix}` - Query vRIB prefix
- `GET /bmp-routers/` - BMP router information
- `GET /rib/` - RIB status and information

### Example Queries
```bash
# Check system metrics
curl http://localhost:8080/metrics

# Query a specific prefix
curl "http://localhost:8080/prefixes/192.168.1.0/24"

# Get RIB information
curl http://localhost:8080/rib/
```