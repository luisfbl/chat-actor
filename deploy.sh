#!/bin/bash
set -e

echo "🚀 Deploy da Aplicação de Chat Completa..."

# Configurar Docker para minikube
if command -v minikube &> /dev/null; then
    echo "🐳 Configurando Docker para Minikube..."
    eval $(minikube docker-env)
    
    # Build todas as imagens
    echo "🔨 Buildando imagens Docker..."
    echo "  - WebSocket Server..."
    docker build -t chat-actor-websocket:latest .
    
    echo "  - API Webserver..."
    docker build -t chat-actor-webserver:latest -f webserver/Dockerfile .
    
    echo "  - Frontend Website..."
    docker build -t chat-actor-frontend:latest -f website/Dockerfile .
    
    echo "✅ Imagens buildadas com sucesso!"
fi

# Deploy PostgreSQL primeiro
echo "🐘 Deploy PostgreSQL..."
kubectl apply -f config/postgres-init-scripts.yaml
kubectl apply -f deployments/postgres-deployment.yaml
kubectl apply -f services/postgres-service.yaml

# Aguardar PostgreSQL
echo "⏳ Aguardando PostgreSQL..."
kubectl wait --for=condition=ready pod -l app=postgres --timeout=180s

# Deploy Redis
echo "📦 Deploy Redis..."
kubectl apply -f redis/

# Aguardar Redis
echo "⏳ Aguardando Redis..."
kubectl wait --for=condition=ready pod -l app=redis --timeout=120s

# Deploy Webserver (API)
echo "🔧 Deploy API Webserver..."
kubectl apply -f deployments/webserver-deployment.yaml
kubectl apply -f services/webserver-service.yaml

# Aguardar Webserver
echo "⏳ Aguardando API Webserver..."
kubectl wait --for=condition=available --timeout=120s deployment/webserver-deployment

# Deploy Frontend
echo "🎨 Deploy Frontend..."
kubectl apply -f deployments/frontend-deployment.yaml
kubectl apply -f services/frontend-service.yaml

# Aguardar Frontend
echo "⏳ Aguardando Frontend..."
kubectl wait --for=condition=available --timeout=120s deployment/frontend-deployment

# Deploy WebSocket
echo "🌐 Deploy WebSocket Server..."
kubectl apply -f config/websocket-config.yaml
kubectl apply -f deployments/websocket-deployment.yaml
kubectl apply -f services/websocket-service.yaml

# Aguardar WebSocket
echo "⏳ Aguardando WebSocket..."
kubectl wait --for=condition=available --timeout=120s deployment/websocket-deployment

# Deploy Ingress e Autoscaling
echo "🌍 Deploy Ingress e Autoscaling..."
kubectl apply -f ingress/
kubectl apply -f autoscaling/

echo "✅ Deploy concluído com sucesso!"
echo ""
echo "📊 Status dos recursos:"
echo "======================"
kubectl get pods -o wide
echo ""
kubectl get services
echo ""
kubectl get ingress
echo ""

echo "🔗 URLs de acesso:"
echo "=================="
if command -v minikube &> /dev/null; then
    MINIKUBE_IP=$(minikube ip)
    echo "🏠 Frontend: http://$MINIKUBE_IP"
    echo "🔌 API: http://$MINIKUBE_IP/api/messages"
    echo "💬 WebSocket: ws://$MINIKUBE_IP/ws/your-username"
    echo ""
    echo "🎯 Para testar a API:"
    echo "curl -X POST http://$MINIKUBE_IP/api/messages \\"
    echo "  -H 'Content-Type: application/json' \\"
    echo "  -d '{\"user_id\": 1, \"content\": \"Hello from API!\"}'"
else
    echo "Frontend: http://localhost"
    echo "API: http://localhost/api/messages"
    echo "WebSocket: ws://localhost/ws/your-username"
fi