if command -v minikube &> /dev/null; then
    echo "Usando minikube..."
    eval $(minikube docker-env)
    docker build -t websocket-service:latest .
fi

# Para kind
if command -v kind &> /dev/null; then
    echo "Usando kind..."
    docker build -t websocket-service:latest .
    kind load docker-image websocket-service:latest --name websocket-cluster
fi

# Deploy Redis primeiro
kubectl apply -f redis/

# Aguardar Redis ficar pronto
echo "Aguardando Redis ficar pronto..."
kubectl wait --for=condition=available --timeout=60s deployment/redis

# Apply das configurações do Kubernetes
kubectl apply -f config/
kubectl apply -f deployments/
kubectl apply -f services/
kubectl apply -f ingress/
kubectl apply -f autoscaling/

# Verificar status
echo "=== STATUS DOS PODS ==="
kubectl get pods -l app=redis
kubectl get pods -l app=websocket-service

echo "=== SERVICES ==="
kubectl get services redis-service
kubectl get services websocket-service

echo "=== INGRESS ==="
kubectl get ingress websocket-ingress

echo "=== LOGS REDIS ==="
kubectl logs deployment/redis --tail=5

echo "=== LOGS WEBSOCKET ==="
kubectl logs deployment/websocket-service --tail=10