# Paper2Codes Deployment Guide

This guide provides instructions for deploying Paper2Codes in production environments.

## Prerequisites

- Docker and Docker Compose installed
- At least 4GB RAM available
- Ports 3000, 8080, 8000 available (or configure custom ports)
- API keys for LLM providers (OpenRouter, OpenAI, Anthropic, etc.)

## Quick Start with Docker Compose

### 1. Clone and Configure

```bash
git clone https://github.com/yourusername/Paper2Codes.git
cd Paper2Codes
```

### 2. Set Environment Variables

Create a `.env` file in the root directory:

```bash
# LLM API Keys
OPENROUTER_API_KEY=sk-or-v1-...
# Or use individual provider keys:
# OPENAI_API_KEY=sk-...
# ANTHROPIC_API_KEY=sk-ant-...

# Database Configuration (optional - defaults work for development)
DATABASE_URL=ws://surreal:8000
DATABASE_NAMESPACE=paper2codes
DATABASE_NAME=main
DATABASE_USER=root
DATABASE_PASS=root

# API Configuration
API_HOST=0.0.0.0
API_PORT=8080
CORS_ORIGINS=http://localhost:3000

# WebUI Configuration
NEXT_PUBLIC_API_URL=http://localhost:8080
NEXT_PUBLIC_WS_URL=ws://localhost:8080/ws
```

### 3. Start Services

```bash
# Start all services
docker-compose up -d

# Start with monitoring stack (Prometheus + Grafana)
docker-compose --profile monitoring up -d

# View logs
docker-compose logs -f

# Check service status
docker-compose ps
```

### 4. Verify Deployment

```bash
# Check API health
curl http://localhost:8080/api/health

# Check WebUI
curl http://localhost:3000

# Check metrics (if monitoring enabled)
curl http://localhost:8080/metrics
```

## Production Deployment

### Using Docker Compose (Recommended for Small-Medium Deployments)

1. **Configure Production Settings**:
   - Update `.env` with production values
   - Set strong database passwords
   - Configure CORS origins for your domain
   - Enable TLS/HTTPS (use reverse proxy)

2. **Use Production Compose Override**:
   ```bash
   docker-compose -f docker-compose.yml -f docker-compose.prod.yml up -d
   ```

3. **Set Up Reverse Proxy** (Nginx/Traefik):
   - Configure SSL certificates
   - Set up domain names
   - Configure rate limiting
   - Enable compression

### Using Kubernetes

1. **Create Namespace**:
   ```bash
   kubectl create namespace paper2codes
   ```

2. **Create Secrets**:
   ```bash
   kubectl create secret generic paper2codes-secrets \
     --from-literal=openrouter-api-key=sk-or-v1-... \
     --from-literal=database-pass=your-secure-password \
     -n paper2codes
   ```

3. **Deploy Services**:
   ```bash
   kubectl apply -f k8s/ -n paper2codes
   ```

4. **Expose Services**:
   ```bash
   kubectl expose deployment paper2codes-core --type=LoadBalancer -n paper2codes
   kubectl expose deployment paper2codes-webui --type=LoadBalancer -n paper2codes
   ```

### Using Cloud Platforms

#### AWS (ECS/EKS)

1. **Build and Push Images**:
   ```bash
   aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin <account-id>.dkr.ecr.us-east-1.amazonaws.com
   docker tag paper2codes/core:latest <account-id>.dkr.ecr.us-east-1.amazonaws.com/paper2codes-core:latest
   docker push <account-id>.dkr.ecr.us-east-1.amazonaws.com/paper2codes-core:latest
   ```

2. **Deploy with ECS**:
   - Create ECS task definitions
   - Configure service discovery
   - Set up load balancers
   - Configure auto-scaling

#### Google Cloud Platform (GKE/Cloud Run)

1. **Build and Push Images**:
   ```bash
   gcloud builds submit --tag gcr.io/<project-id>/paper2codes-core
   ```

2. **Deploy to Cloud Run**:
   ```bash
   gcloud run deploy paper2codes-core \
     --image gcr.io/<project-id>/paper2codes-core \
     --platform managed \
     --region us-central1
   ```

## Monitoring Setup

### Prometheus

1. **Access Prometheus**:
   - URL: http://localhost:9090 (with monitoring profile)
   - Query metrics: `paper2codes_api_http_requests_total`

2. **Configure Alerts** (optional):
   - Edit `monitoring/prometheus.yml`
   - Add alert rules
   - Configure AlertManager

### Grafana

1. **Access Grafana**:
   - URL: http://localhost:3001
   - Default credentials: admin/admin

2. **Import Dashboards**:
   - Prometheus datasource is auto-configured
   - Create custom dashboards or import from community

3. **Set Up Alerts**:
   - Configure notification channels
   - Create alert rules
   - Set up PagerDuty/Slack integration

## Security Hardening

### 1. Environment Variables
- Never commit `.env` files
- Use secret management (AWS Secrets Manager, HashiCorp Vault)
- Rotate API keys regularly

### 2. Network Security
- Use reverse proxy (Nginx/Traefik) for TLS termination
- Configure firewall rules
- Use private networks for internal services
- Enable rate limiting

### 3. Container Security
- Run containers as non-root users (already configured)
- Regularly update base images
- Scan images for vulnerabilities
- Use minimal base images

### 4. Database Security
- Use strong passwords
- Enable TLS for database connections
- Restrict network access
- Regular backups

### 5. API Security
- Enable authentication for all endpoints
- Use HTTPS in production
- Configure CORS properly
- Enable rate limiting
- Protect metrics endpoint (add authentication)

## Scaling

### Horizontal Scaling

1. **API Server**:
   - Deploy multiple Core instances
   - Use load balancer
   - Share SurrealDB instance
   - Use Redis for distributed caching

2. **WebUI**:
   - Deploy multiple WebUI instances
   - Use CDN for static assets
   - Configure session storage (if needed)

### Vertical Scaling

1. **Increase Resources**:
   - Allocate more CPU/memory to containers
   - Optimize database queries
   - Enable connection pooling

2. **Performance Tuning**:
   - Adjust worker threads
   - Configure cache sizes
   - Optimize LLM request batching

## Backup and Recovery

### Database Backups

```bash
# Backup SurrealDB
docker exec paper2codes-surreal surreal export --conn ws://localhost:8000 --user root --pass root --ns paper2codes --db main backup.surql

# Restore from backup
docker exec -i paper2codes-surreal surreal import --conn ws://localhost:8000 --user root --pass root --ns paper2codes --db main < backup.surql
```

### Automated Backups

Set up cron job or scheduled task:
```bash
0 2 * * * docker exec paper2codes-surreal surreal export --conn ws://localhost:8000 --user root --pass root --ns paper2codes --db main /backups/backup-$(date +\%Y\%m\%d).surql
```

## Troubleshooting

### Service Won't Start

1. **Check Logs**:
   ```bash
   docker-compose logs core
   docker-compose logs webui
   docker-compose logs surreal
   ```

2. **Check Ports**:
   ```bash
   netstat -tulpn | grep -E '3000|8080|8000'
   ```

3. **Check Resources**:
   ```bash
   docker stats
   ```

### Database Connection Issues

1. **Verify SurrealDB is Running**:
   ```bash
   docker-compose ps surreal
   curl http://localhost:8000/health
   ```

2. **Check Connection String**:
   - Verify `DATABASE_URL` in `.env`
   - Check network connectivity
   - Verify credentials

### Performance Issues

1. **Check Metrics**:
   - View Prometheus metrics
   - Check Grafana dashboards
   - Analyze slow queries

2. **Optimize Configuration**:
   - Adjust connection pool sizes
   - Enable caching
   - Optimize LLM request batching

## Maintenance

### Updates

1. **Pull Latest Changes**:
   ```bash
   git pull origin main
   ```

2. **Rebuild and Restart**:
   ```bash
   docker-compose build
   docker-compose up -d
   ```

3. **Verify Health**:
   ```bash
   curl http://localhost:8080/api/health
   ```

### Log Rotation

Configure log rotation in `docker-compose.yml`:
```yaml
logging:
  driver: "json-file"
  options:
    max-size: "10m"
    max-file: "3"
```

## Support

For issues and questions:
- GitHub Issues: https://github.com/yourusername/Paper2Codes/issues
- Documentation: See [README.md](README.md) and [INTEGRATION.md](INTEGRATION.md)
- API Documentation: See `Paper2Codes-Core/openapi.yaml`

