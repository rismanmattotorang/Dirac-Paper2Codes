# Paper2Codes Kubernetes Deployment

This directory contains Kubernetes manifests for deploying Paper2Codes to a Kubernetes cluster.

## Prerequisites

- Kubernetes cluster (v1.24+)
- `kubectl` configured to access your cluster
- Container registry access (Docker Hub, GitHub Container Registry, or private registry)
- TLS certificates (for production)
- Ingress controller (NGINX recommended)

## Quick Start

### 1. Create Namespace

```bash
kubectl apply -f namespace.yaml
```

### 2. Configure Secrets

**⚠️ IMPORTANT: Do NOT use the example secrets file in production!**

Create secrets using kubectl:

```bash
# Create secrets from literal values
kubectl create secret generic paper2codes-secrets \
  --from-literal=OPENROUTER_API_KEY="sk-or-v1-..." \
  --from-literal=DATABASE_PASS="secure-password" \
  --from-literal=JWT_SECRET_KEY="secure-random-string" \
  -n paper2codes

# Or from a file (recommended)
kubectl create secret generic paper2codes-secrets \
  --from-file=.env.production \
  -n paper2codes
```

### 3. Apply ConfigMap

```bash
kubectl apply -f configmap.yaml
```

### 4. Deploy Services

```bash
# Deploy in order
kubectl apply -f surrealdb-deployment.yaml
kubectl apply -f core-deployment.yaml
kubectl apply -f webui-deployment.yaml
kubectl apply -f ingress.yaml
```

### 5. Verify Deployment

```bash
# Check pods
kubectl get pods -n paper2codes

# Check services
kubectl get services -n paper2codes

# Check ingress
kubectl get ingress -n paper2codes

# View logs
kubectl logs -f deployment/paper2codes-core -n paper2codes
kubectl logs -f deployment/paper2codes-webui -n paper2codes
```

## Deployment Files

| File | Description |
|------|-------------|
| `namespace.yaml` | Creates the paper2codes namespace |
| `configmap.yaml` | Non-sensitive configuration |
| `secrets.yaml` | Example secrets (NOT for production) |
| `surrealdb-deployment.yaml` | SurrealDB database deployment with PVC |
| `core-deployment.yaml` | Backend API server deployment + HPA |
| `webui-deployment.yaml` | Frontend web UI deployment + HPA |
| `ingress.yaml` | Ingress configuration for external access |

## Configuration

### Environment Variables

See `env.example` files in backend and frontend directories for all available configuration options.

### Resource Limits

**Backend (Core):**
- Requests: 1GB RAM, 500m CPU
- Limits: 4GB RAM, 2 CPU
- Auto-scaling: 3-10 replicas

**Frontend (WebUI):**
- Requests: 256MB RAM, 200m CPU
- Limits: 1GB RAM, 1 CPU
- Auto-scaling: 2-5 replicas

**Database (SurrealDB):**
- Requests: 512MB RAM, 500m CPU
- Limits: 2GB RAM, 2 CPU
- Storage: 10GB PVC

### Auto-Scaling

Horizontal Pod Autoscalers (HPA) are configured for both Core and WebUI:

**Core HPA:**
- Min replicas: 3
- Max replicas: 10
- Target CPU: 70%
- Target Memory: 80%

**WebUI HPA:**
- Min replicas: 2
- Max replicas: 5
- Target CPU: 70%
- Target Memory: 80%

## Production Deployment

### 1. Update Docker Images

Update image references in deployment files:

```yaml
# core-deployment.yaml and webui-deployment.yaml
image: your-registry/paper2codes/core:version
image: your-registry/paper2codes/webui:version
```

### 2. Configure TLS

Create TLS secret for HTTPS:

```bash
# Using cert-manager (recommended)
kubectl apply -f - <<EOF
apiVersion: cert-manager.io/v1
kind: Certificate
metadata:
  name: paper2codes-tls
  namespace: paper2codes
spec:
  secretName: paper2codes-tls
  issuerRef:
    name: letsencrypt-prod
    kind: ClusterIssuer
  dnsNames:
  - paper2codes.example.com
  - api.paper2codes.example.com
EOF

# Or manually create from certificates
kubectl create secret tls paper2codes-tls \
  --cert=path/to/tls.crt \
  --key=path/to/tls.key \
  -n paper2codes
```

### 3. Update Ingress

Update `ingress.yaml` with your domain names and TLS configuration.

### 4. Configure Persistent Storage

For production, use a storage class with backups:

```yaml
# In surrealdb-deployment.yaml
spec:
  storageClassName: fast-ssd  # Your storage class
  resources:
    requests:
      storage: 50Gi  # Adjust as needed
```

### 5. Set Up Monitoring

Deploy Prometheus and Grafana for monitoring:

```bash
# Add Prometheus Helm repo
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm repo update

# Install Prometheus
helm install prometheus prometheus-community/kube-prometheus-stack \
  --namespace monitoring --create-namespace

# Import Paper2Codes dashboards
kubectl apply -f ../monitoring/grafana/dashboards/
```

### 6. Configure Backup

Set up automated backups for SurrealDB:

```bash
# Create CronJob for backups
kubectl apply -f backup-cronjob.yaml
```

## Monitoring

### Health Checks

**Backend:**
- Liveness: `GET /api/health`
- Readiness: `GET /api/health`
- Startup: `GET /api/health` (30 retries, 10s interval)

**Frontend:**
- Liveness: `GET /`
- Readiness: `GET /`

**Database:**
- Liveness: `GET /health`
- Readiness: `GET /health`

### Metrics

Prometheus metrics are exposed on:
- Backend: `http://paper2codes-core:9090/metrics`

### Logs

View logs:

```bash
# Backend logs
kubectl logs -f deployment/paper2codes-core -n paper2codes

# Frontend logs
kubectl logs -f deployment/paper2codes-webui -n paper2codes

# Database logs
kubectl logs -f deployment/surreal -n paper2codes

# All pods
kubectl logs -f -l app.kubernetes.io/name=paper2codes -n paper2codes
```

## Troubleshooting

### Pods Not Starting

```bash
# Describe pod for events
kubectl describe pod <pod-name> -n paper2codes

# Check events
kubectl get events -n paper2codes --sort-by='.lastTimestamp'

# Check logs
kubectl logs <pod-name> -n paper2codes --previous
```

### Database Connection Issues

```bash
# Check SurrealDB pod
kubectl get pod -l app=surreal -n paper2codes

# Test connection from Core pod
kubectl exec -it deployment/paper2codes-core -n paper2codes -- \
  curl http://surreal:8000/health

# Check service endpoints
kubectl get endpoints -n paper2codes
```

### Ingress Issues

```bash
# Check ingress status
kubectl describe ingress paper2codes-ingress -n paper2codes

# Check ingress controller logs
kubectl logs -f -n ingress-nginx deployment/ingress-nginx-controller
```

### Performance Issues

```bash
# Check resource usage
kubectl top pods -n paper2codes
kubectl top nodes

# Check HPA status
kubectl get hpa -n paper2codes
kubectl describe hpa paper2codes-core-hpa -n paper2codes
```

## Scaling

### Manual Scaling

```bash
# Scale Core deployment
kubectl scale deployment/paper2codes-core --replicas=5 -n paper2codes

# Scale WebUI deployment
kubectl scale deployment/paper2codes-webui --replicas=3 -n paper2codes
```

### Update HPA Limits

Edit HPA manifests and reapply:

```bash
kubectl apply -f core-deployment.yaml
kubectl apply -f webui-deployment.yaml
```

## Updates and Rollbacks

### Rolling Update

```bash
# Update image
kubectl set image deployment/paper2codes-core \
  paper2codes-core=your-registry/paper2codes/core:v1.1.0 \
  -n paper2codes

# Watch rollout
kubectl rollout status deployment/paper2codes-core -n paper2codes
```

### Rollback

```bash
# Rollback to previous version
kubectl rollout undo deployment/paper2codes-core -n paper2codes

# Rollback to specific revision
kubectl rollout undo deployment/paper2codes-core --to-revision=2 -n paper2codes

# Check rollout history
kubectl rollout history deployment/paper2codes-core -n paper2codes
```

## Security

### Network Policies

Apply network policies to restrict traffic:

```bash
kubectl apply -f network-policies.yaml
```

### Pod Security

Enable Pod Security Standards:

```bash
kubectl label namespace paper2codes \
  pod-security.kubernetes.io/enforce=baseline \
  pod-security.kubernetes.io/audit=restricted \
  pod-security.kubernetes.io/warn=restricted
```

### RBAC

Create service accounts with minimal permissions:

```bash
kubectl apply -f rbac.yaml
```

## Backup and Restore

### Backup Database

```bash
# Manual backup
kubectl exec deployment/surreal -n paper2codes -- \
  surreal export --conn ws://localhost:8000 \
  --user root --pass $DB_PASS \
  --ns paper2codes --db main \
  /tmp/backup.surql

# Copy backup locally
kubectl cp paper2codes/surreal-pod:/tmp/backup.surql ./backup-$(date +%Y%m%d).surql
```

### Restore Database

```bash
# Copy backup to pod
kubectl cp ./backup.surql paper2codes/surreal-pod:/tmp/backup.surql

# Restore
kubectl exec deployment/surreal -n paper2codes -- \
  surreal import --conn ws://localhost:8000 \
  --user root --pass $DB_PASS \
  --ns paper2codes --db main \
  /tmp/backup.surql
```

## Cleanup

### Delete All Resources

```bash
# Delete deployments
kubectl delete -f .

# Delete namespace (removes everything)
kubectl delete namespace paper2codes
```

## Support

For issues and questions:
- GitHub Issues: https://github.com/yourusername/Paper2Codes/issues
- Documentation: See main README.md
- Deployment Guide: See DEPLOYMENT.md

