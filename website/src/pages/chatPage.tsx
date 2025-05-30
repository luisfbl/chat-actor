import React, { useState, useRef, useEffect } from "react";
import "../assets/css/chat.css";

interface Message {
  id: number;
  user: string;
  text: string;
}

const ChatScreen: React.FC = () => {
  const [messages, setMessages] = useState<Message[]>([
    { id: 1, user: "Alice", text: "Olá, pessoal!" },
    { id: 2, user: "Bob", text: "E aí, tudo bem?" },
  ]);
  const [input, setInput] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const users = ["Alice", "Bob", "Carol", "Dave"];

  const sendMessage = () => {
    if (!input.trim()) return;
    const next: Message = {
      id: messages.length + 1,
      user: "Você",
      text: input,
    };
    setMessages([...messages, next]);
    setInput("");
  };

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  return (
    <div className="chat-container">
      <div className="chat-main">
        <header className="chat-header"># geral</header>
        <div className="chat-messages">
          {messages.map((msg) => (
            <div key={msg.id} className="message">
              <span className="message-user">{msg.user}:</span>{" "}
              <span className="message-text">{msg.text}</span>
            </div>
          ))}
          <div ref={messagesEndRef} />
        </div>
        <div className="chat-input">
          <input
            type="text"
            placeholder="Digite sua mensagem..."
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && sendMessage()}
          />
        </div>
      </div>
      <aside className="chat-sidebar">
        <header className="sidebar-header">Pessoas</header>
        <ul className="user-list">
          {users.map((u) => (
            <li key={u} className="user-item">
              {u}
            </li>
          ))}
        </ul>
      </aside>
    </div>
  );
};

export default ChatScreen;
