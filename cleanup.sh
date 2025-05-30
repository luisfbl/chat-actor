#!/bin/bash
# quick-cleanup.sh - Limpeza rápida apenas dos recursos do projeto

set -e

echo "🚀 Limpeza rápida do websocket..."

# Configurar minikube se disponível
if command -v minikube &> /dev/null && minikube status &> /dev/null; then
    eval $(minikube docker-env) 2>/dev/null || true
fi

# Remover recursos Kubernetes
echo "Removendo recursos K8s..."
kubectl delete hpa websocket-hpa 2>/dev/null || true
kubectl delete ingress websocket-ingress 2>/dev/null || true
kubectl delete service websocket-service 2>/dev/null || true
kubectl delete deployment websocket-service 2>/dev/null || true
kubectl delete configmap websocket-config 2>/dev/null || true

# Aguardar pods serem removidos
kubectl wait --for=delete pods -l app=websocket-service --timeout=30s 2>/dev/null || true

# Remover imagem
echo "Removendo imagem..."
docker rmi -f websocket-service:latest 2>/dev/null || true

echo "✅ Limpeza rápida concluída!"
echo "Para verificar: kubectl get all"