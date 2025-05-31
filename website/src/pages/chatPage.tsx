import React, { useState, useRef, useEffect, type FormEvent } from "react";
import { useWebSocket } from "../hooks/useWebSocket";
import "../assets/css/chat.css";
import defaultAvatar from "../assets/defaultAvatar.png";

interface WSMessage {
  id: number;
  user: string;
  text: string;
  timestamp: Date;
}

interface ChatScreenProps {
  userName: string;
}

const ChatScreen: React.FC<ChatScreenProps> = ({ userName }) => {
  const [input, setInput] = useState("");
  const { messages, isConnected, error, sendMessage } = useWebSocket(userName);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const handleSendMessage = (e: FormEvent) => {
    e.preventDefault();
    if (!input.trim()) return;
    if (sendMessage(input.trim())) {
      setInput("");
    }
  };

  return (
    <div className="chat-container">
      <div className="user-header">
        <div className="user-info-wrapper">
          <span className="user-name-header">{userName}</span>
          <img
            src={defaultAvatar}
            alt="Avatar do usuário"
            className="user-avatar"
          />
        </div>
      </div>

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
              <p>Bem-vindo ao chat, {userName}! 👋</p>
              <p>Digite uma mensagem para começar a conversar.</p>
            </div>
          ) : (
            messages.map((msg) => (
              <div
                key={msg.id}
                className={`message-bubble ${
                  msg.user === userName ? "right" : "left"
                }`}
              >
                <div className="message-meta">
                  <span className="user-name">{msg.user}</span>
                  <span className="time">
                    {msg.timestamp.toLocaleTimeString([], {
                      hour: "2-digit",
                      minute: "2-digit",
                    })}
                  </span>
                </div>
                <div className="message-text">{msg.text}</div>
              </div>
            ))
          )}
          <div ref={messagesEndRef} />
        </div>

        <form className="chat-input" onSubmit={handleSendMessage}>
          <input
            type="text"
            placeholder={
              isConnected ? "Digite sua mensagem..." : "Conectando..."
            }
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
            <span className="username">{userName}</span>
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
