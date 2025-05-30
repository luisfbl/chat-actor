import React, { useState, useRef, useEffect } from "react";
import { useWebSocket } from "../hooks/useWebSocket";
import "../assets/css/chat.css";

const ChatScreen: React.FC = () => {
  const [username, setUsername] = useState<string>("");
  const [input, setInput] = useState("");
  const [isUsernameSet, setIsUsernameSet] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const { messages, isConnected, error, sendMessage } = useWebSocket(username);

  const handleSetUsername = (e: React.FormEvent) => {
    e.preventDefault();
    if (username.trim()) {
      setIsUsernameSet(true);
    }
  };

  const handleSendMessage = (e: React.FormEvent) => {
    e.preventDefault();
    if (input.trim() && sendMessage(input)) {
      setInput("");
    }
  };

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // Tela de entrada do usuário
  if (!isUsernameSet) {
    return (
        <div className="login-container">
          <div className="login-form">
            <h2>Digite seu nome de usuário</h2>
            <form onSubmit={handleSetUsername}>
              <input
                  type="text"
                  placeholder="Nome de usuário"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  autoFocus
              />
              <button type="submit" disabled={!username.trim()}>
                Entrar no Chat
              </button>
            </form>
          </div>
        </div>
    );
  }

  return (
      <div className="chat-container">
        <div className="chat-main">
          <header className="chat-header">
            <span># geral</span>
            <div className="connection-status">
              {isConnected ? (
                  <span className="status-connected">🟢 Conectado</span>
              ) : (
                  <span className="status-disconnected">🔴 Desconectado</span>
              )}
              {error && <span className="status-error">⚠️ {error}</span>}
            </div>
          </header>

          <div className="chat-messages">
            {messages.length === 0 ? (
                <div className="welcome-message">
                  <p>Bem-vindo ao chat, {username}! 👋</p>
                  <p>Digite uma mensagem para começar a conversar.</p>
                </div>
            ) : (
                messages.map((msg) => (
                    <div
                        key={msg.id}
                        className={`message ${msg.user === username ? 'own-message' : ''}`}
                    >
                      <span className="message-user">{msg.user}:</span>{" "}
                      <span className="message-text">{msg.text}</span>
                      <span className="message-time">
                  {msg.timestamp.toLocaleTimeString()}
                </span>
                    </div>
                ))
            )}
            <div ref={messagesEndRef} />
          </div>

          <form className="chat-input" onSubmit={handleSendMessage}>
            <input
                type="text"
                placeholder={isConnected ? "Digite sua mensagem..." : "Conectando..."}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                disabled={!isConnected}
            />
            <button type="submit" disabled={!isConnected || !input.trim()}>
              Enviar
            </button>
          </form>
        </div>

        <aside className="chat-sidebar">
          <header className="sidebar-header">
            <span>Usuário Logado</span>
          </header>
          <div className="current-user">
            <div className="user-info">
              <span className="username">{username}</span>
              <span className="user-status">
              {isConnected ? "Online" : "Offline"}
            </span>
            </div>
          </div>

          <header className="sidebar-header">
            <span>Debug Info</span>
          </header>
          <div className="debug-info">
            <p>Status: {isConnected ? "Conectado" : "Desconectado"}</p>
            <p>Mensagens: {messages.length}</p>
            {error && <p className="error">Erro: {error}</p>}
          </div>
        </aside>
      </div>
  );
};

export default ChatScreen;