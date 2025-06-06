#!/bin/bash
# cleanup.sh - Limpeza completa da aplicação de chat

set -e

echo "🧹 Limpeza completa da aplicação de chat..."

# Configurar minikube se disponível
if command -v minikube &> /dev/null && minikube status &> /dev/null; then
    eval $(minikube docker-env) 2>/dev/null || true
fi

# Remover todos os recursos Kubernetes
echo "🗑️ Removendo recursos Kubernetes..."

# Autoscaling
kubectl delete hpa websocket-hpa 2>/dev/null || true

# Ingress
kubectl delete ingress chat-app-ingress 2>/dev/null || true
kubectl delete ingress websocket-ingress 2>/dev/null || true

# Services
kubectl delete service frontend-service 2>/dev/null || true
kubectl delete service webserver-service 2>/dev/null || true
kubectl delete service websocket-service 2>/dev/null || true
kubectl delete service postgres-service 2>/dev/null || true
kubectl delete service redis-service 2>/dev/null || true

# Deployments
kubectl delete deployment frontend-deployment 2>/dev/null || true
kubectl delete deployment webserver-deployment 2>/dev/null || true
kubectl delete deployment websocket-deployment 2>/dev/null || true
kubectl delete deployment websocket-service 2>/dev/null || true
kubectl delete deployment postgres-deployment 2>/dev/null || true
kubectl delete deployment redis 2>/dev/null || true

# ConfigMaps
kubectl delete configmap websocket-config 2>/dev/null || true
kubectl delete configmap postgres-init-scripts 2>/dev/null || true

# PVC (cuidado - isso remove dados persistentes)
echo "⚠️ Removendo dados persistentes..."
kubectl delete pvc postgres-pvc 2>/dev/null || true

# ServiceMonitor
kubectl delete servicemonitor websocket-monitor 2>/dev/null || true

# Aguardar pods serem removidos
echo "⏳ Aguardando pods serem removidos..."
kubectl wait --for=delete pods -l app=frontend --timeout=30s 2>/dev/null || true
kubectl wait --for=delete pods -l app=webserver --timeout=30s 2>/dev/null || true
kubectl wait --for=delete pods -l app=websocket --timeout=30s 2>/dev/null || true
kubectl wait --for=delete pods -l app=postgres --timeout=30s 2>/dev/null || true
kubectl wait --for=delete pods -l app=redis --timeout=30s 2>/dev/null || true

# Remover imagens Docker
echo "🐳 Removendo imagens Docker..."
docker rmi -f chat-actor-frontend:latest 2>/dev/null || true
docker rmi -f chat-actor-webserver:latest 2>/dev/null || true
docker rmi -f chat-actor-websocket:latest 2>/dev/null || true
docker rmi -f websocket-service:latest 2>/dev/null || true

echo "✅ Limpeza completa concluída!"
echo ""
echo "📊 Status final:"
echo "==============="
kubectl get all
echo ""
echo "🔄 Para fazer deploy novamente, execute: ./deploy.sh"