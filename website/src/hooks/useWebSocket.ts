import { useState, useEffect, useRef, useCallback } from 'react';

export interface Message {
    id: number;
    user: string;
    text: string;
    timestamp: Date;
}

export interface WebSocketMessage {
    username: string;
    content: string;
}

export interface JoinEvent {
    username: string;
}

export interface LeaveEvent {
    username: string;
}

export const useWebSocket = (username: string) => {
    const [messages, setMessages] = useState<Message[]>([]);
    const [connectedUsers] = useState<string[]>([]);
    const [isConnected, setIsConnected] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const ws = useRef<WebSocket | null>(null);
    const messageIdCounter = useRef(1);

    // Determinar URL do WebSocket baseado no ambiente
    const getWebSocketUrl = useCallback(() => {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const host = window.location.hostname;

        // Se estiver rodando em desenvolvimento local
        if (host === 'localhost' || host === '127.0.0.1') {
            // Assumir que minikube está rodando na porta padrão
            return `ws://192.168.49.2/ws/${encodeURIComponent(username)}`;
        }

        // Para produção ou outros ambientes
        return `${protocol}//${host}/ws/${encodeURIComponent(username)}`;
    }, [username]);

    const connect = useCallback(() => {
        if (ws.current?.readyState === WebSocket.OPEN) {
            return;
        }

        try {
            const wsUrl = getWebSocketUrl();
            console.log('Conectando ao WebSocket:', wsUrl);

            ws.current = new WebSocket(wsUrl);

            ws.current.onopen = () => {
                console.log('WebSocket conectado');
                setIsConnected(true);
                setError(null);
            };

            ws.current.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    console.log('Mensagem recebida:', data);

                    if (data.username && data.content) {
                        const newMessage: Message = {
                            id: messageIdCounter.current++,
                            user: data.username,
                            text: data.content,
                            timestamp: new Date(),
                        };
                        setMessages(prev => [...prev, newMessage]);
                    } else if (data.username && !data.content) {
                        console.log('Evento de usuário:', data);
                    }
                } catch (err) {
                    console.error('Erro ao processar mensagem:', err);
                }
            };

            ws.current.onclose = (event) => {
                console.log('WebSocket desconectado:', event.code, event.reason);
                setIsConnected(false);

                // Reconectar automaticamente após 3 segundos se não foi fechamento intencional
                if (event.code !== 1000) {
                    setTimeout(() => {
                        console.log('Tentando reconectar...');
                        connect();
                    }, 3000);
                }
            };

            ws.current.onerror = (error) => {
                console.error('Erro WebSocket:', error);
                setError('Erro na conexão WebSocket');
                setIsConnected(false);
            };
        } catch (err) {
            console.error('Erro ao criar WebSocket:', err);
            setError('Falha ao conectar');
        }
    }, [getWebSocketUrl]);

    const disconnect = useCallback(() => {
        if (ws.current) {
            ws.current.close(1000, 'Desconexão intencional');
            ws.current = null;
        }
    }, []);

    const sendMessage = useCallback((content: string) => {
        if (ws.current?.readyState === WebSocket.OPEN && content.trim()) {
            const message: WebSocketMessage = {
                username,
                content: content.trim(),
            };

            console.log('Enviando mensagem:', message);
            ws.current.send(JSON.stringify(message));

            // Adicionar mensagem própria à lista local
            const newMessage: Message = {
                id: messageIdCounter.current++,
                user: username,
                text: content.trim(),
                timestamp: new Date(),
            };
            setMessages(prev => [...prev, newMessage]);

            return true;
        }
        return false;
    }, [username]);

    // Conectar automaticamente quando o hook é usado
    useEffect(() => {
        if (username) {
            connect();
        }

        return () => {
            disconnect();
        };
    }, [username, connect, disconnect]);

    return {
        messages,
        connectedUsers,
        isConnected,
        error,
        sendMessage,
        connect,
        disconnect,
    };
};