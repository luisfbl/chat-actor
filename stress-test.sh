#!/bin/bash

# Configurações padrão
NUM_CLIENTS=${1:-50}
DURATION_MINUTES=${2:-15}
MESSAGE_INTERVAL=${3:-5}
WS_URL=${4:-"ws://192.168.49.2"}

echo "🔥 CHAT-ACTOR STRESS TEST"
echo "========================"
echo "📊 Configuração:"
echo "   - Clientes: $NUM_CLIENTS"
echo "   - Duração: $DURATION_MINUTES minutos"
echo "   - Intervalo de mensagens: $MESSAGE_INTERVAL segundos"
echo "   - URL: $WS_URL"
echo ""

# Calcular duração em segundos
DURATION_SECONDS=$((DURATION_MINUTES * 60))
END_TIME=$(($(date +%s) + DURATION_SECONDS))

# Diretório para logs dos clientes
LOG_DIR="/tmp/stress-test-logs"
mkdir -p "$LOG_DIR"

# Função para conectar um cliente WebSocket usando websocat
connect_client() {
    local client_id=$1
    local username="Bot_$client_id"
    local log_file="$LOG_DIR/client_$client_id.log"
    
    echo "[Cliente $client_id] Iniciando conexão como $username" >> "$log_file"
    
    # Usar wscat para conectar e enviar mensagens periodicamente
    {
        while [ $(date +%s) -lt $END_TIME ]; do
            # Criar mensagem JSON
            local timestamp=$(date +%s)
            local message_content="Mensagem de teste do $username - $(date)"
            local json_message="{\"username\":\"$username\",\"content\":\"$message_content\"}"
            
            echo "$json_message"
            echo "[Cliente $client_id] Enviou: $message_content" >> "$log_file"
            
            # Intervalo com variação aleatória (±2 segundos)
            local variation=$((RANDOM % 4 - 2))
            local sleep_time=$((MESSAGE_INTERVAL + variation))
            [ $sleep_time -lt 1 ] && sleep_time=1
            
            sleep $sleep_time
        done
    } | wscat -c "$WS_URL/ws/$username" 2>>"$log_file" &
    
    local pid=$!
    echo $pid > "$LOG_DIR/client_${client_id}.pid"
    echo "[Cliente $client_id] PID: $pid"
}

# Função para monitorar métricas
monitor_metrics() {
    echo "📈 Iniciando monitoramento de métricas..."
    
    while [ $(date +%s) -lt $END_TIME ]; do
        echo ""
        echo "📊 ===== MÉTRICAS DO TESTE $(date) ====="
        
        # Contar processos ativos
        local active_clients=$(ls "$LOG_DIR"/*.pid 2>/dev/null | wc -l)
        echo "🔗 Clientes ativos: $active_clients/$NUM_CLIENTS"
        
        # Tempo restante
        local remaining=$((END_TIME - $(date +%s)))
        local remaining_min=$((remaining / 60))
        local remaining_sec=$((remaining % 60))
        echo "⏱️  Tempo restante: ${remaining_min}m ${remaining_sec}s"
        
        # Métricas do cluster
        echo "☸️  Métricas do cluster:"
        ./metrics.sh 2>/dev/null | grep -E "(Total Connections|Active Relays|Status)" || echo "   Erro ao obter métricas do cluster"
        
        # Status dos pods
        echo "📦 Status dos pods:"
        kubectl get pods -l app=websocket --no-headers 2>/dev/null | awk '{print "   " $1 ": " $3}' || echo "   Erro ao obter status dos pods"
        
        # HPA status
        echo "📈 HPA:"
        kubectl get hpa websocket-hpa --no-headers 2>/dev/null | awk '{print "   Réplicas: " $6 " (min: " $4 ", max: " $5 ")"}' || echo "   Erro ao obter HPA"
        
        echo "================================"
        
        sleep 30
    done
}

# Função para cleanup
cleanup() {
    echo ""
    echo "🛑 Finalizando teste de estresse..."
    
    # Matar todos os processos dos clientes
    for pid_file in "$LOG_DIR"/*.pid; do
        if [ -f "$pid_file" ]; then
            local pid=$(cat "$pid_file")
            if kill -0 "$pid" 2>/dev/null; then
                kill "$pid" 2>/dev/null
                echo "Finalizando cliente PID: $pid"
            fi
            rm -f "$pid_file"
        fi
    done
    
    # Relatório final
    echo ""
    echo "🏁 ===== RELATÓRIO FINAL ====="
    echo "⏱️  Duração configurada: $DURATION_MINUTES minutos"
    echo "👥 Clientes configurados: $NUM_CLIENTS"
    
    # Contar mensagens enviadas
    local total_messages=$(grep -h "Enviou:" "$LOG_DIR"/*.log 2>/dev/null | wc -l)
    echo "📤 Total de mensagens enviadas: $total_messages"
    
    if [ $total_messages -gt 0 ]; then
        local rate=$(echo "scale=2; $total_messages / ($DURATION_MINUTES * 60)" | bc -l 2>/dev/null || echo "N/A")
        echo "🔄 Taxa média: $rate msg/s"
    fi
    
    # Verificar se há logs de erro
    local errors=$(grep -i "error\|erro\|failed\|falhou" "$LOG_DIR"/*.log 2>/dev/null | wc -l)
    echo "❌ Erros encontrados: $errors"
    
    echo ""
    echo "✅ Teste concluído!"
    echo "📁 Logs salvos em: $LOG_DIR"
    echo "💡 Use ./metrics.sh para verificar métricas finais do cluster"
    
    exit 0
}

# Verificar dependências
if ! command -v wscat >/dev/null 2>&1; then
    echo "❌ wscat não encontrado. Instalando..."
    if command -v npm >/dev/null 2>&1; then
        sudo npm install -g wscat >/dev/null 2>&1 || {
            echo "❌ Não foi possível instalar wscat. Instale manualmente:"
            echo "   sudo npm install -g wscat"
            exit 1
        }
    else
        echo "❌ npm não encontrado. Instale wscat manualmente:"
        echo "   sudo npm install -g wscat"
        exit 1
    fi
fi

if ! command -v bc >/dev/null 2>&1; then
    echo "📦 Instalando bc para cálculos..."
    sudo apt-get update -qq && sudo apt-get install -y bc >/dev/null 2>&1 || {
        echo "⚠️  bc não disponível, algumas métricas podem não funcionar"
    }
fi

# Configurar trap para cleanup
trap cleanup SIGINT SIGTERM EXIT

# Limpar logs anteriores
rm -f "$LOG_DIR"/*

echo "🚀 Iniciando $NUM_CLIENTS clientes..."

# Conectar clientes em lotes para evitar sobrecarga
batch_size=10
for ((i=1; i<=NUM_CLIENTS; i++)); do
    connect_client $i
    
    # A cada lote, fazer uma pausa
    if [ $((i % batch_size)) -eq 0 ]; then
        echo "✅ Conectados $i/$NUM_CLIENTS clientes"
        sleep 2
    fi
done

echo "🎯 Todos os clientes iniciados! Teste rodará por $DURATION_MINUTES minutos..."
echo "💡 Pressione Ctrl+C para finalizar antecipadamente"

# Iniciar monitoramento
monitor_metrics

# O script terminará quando o tempo acabar ou o usuário pressionar Ctrl+C