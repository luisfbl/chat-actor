#!/bin/bash

echo "📊 MÉTRICAS RÁPIDAS - TODOS OS PODS"

# Obter lista de pods
PODS=$(kubectl get pods -l app=websocket -o jsonpath='{.items[*].metadata.name}')

if [ -z "$PODS" ]; then
    echo "❌ Nenhum pod WebSocket encontrado"
    exit 1
fi

echo "🔍 Pods encontrados: $PODS"
echo ""

# Para cada pod
for POD in $PODS; do
    echo "📦 === POD: $POD ==="

    # Port-forward em background
    kubectl port-forward pod/$POD 9002:9002 >/dev/null 2>&1 &
    PF_PID=$!

    # Aguardar conexão
    sleep 2

    # Coletar métricas
    echo "🏥 Health:"
    curl -s "http://localhost:9002/health" 2>/dev/null | jq -r '
        "Status: " + (.status // "unknown") +
        " | Pod ID: " + (.pod_id // "unknown") +
        " | Cluster Pods: " + (.cluster_pods // 0 | tostring)' || echo "Erro ao obter health"

    echo "📊 Métricas:"
    curl -s "http://localhost:9002/metrics" 2>/dev/null | jq -r '
        "Timestamp: " + (.timestamp // 0 | tostring) +
        " | Total Connections: " + (
            .pod_metrics // {} |
            to_entries |
            map(.value.active_connections // 0) |
            add // 0 | tostring
        )' || echo "Erro ao obter métricas"

    echo "🔄 Relays:"
    curl -s "http://localhost:9002/relays" 2>/dev/null | jq -r '
        "Active Relays: " + (.active_relays // [] | length | tostring) +
        " | Pod ID: " + (.pod_id // "unknown")' || echo "Erro ao obter relays"

    # Parar port-forward
    kill $PF_PID 2>/dev/null

    echo ""
    echo "----------------------------------------"
done

echo ""
echo "☸️  Status dos Pods:"
kubectl get pods -l app=websocket -o wide

echo ""
echo "📈 HPA Status:"
kubectl get hpa websocket-hpa

echo ""
echo "💻 Recursos (se metrics-server disponível):"
kubectl top pods -l app=websocket 2>/dev/null || echo "Metrics server não disponível"