import React, { useState } from "react";
import "../assets/css/nameInput.css";

interface NameInputPageProps {
  onSubmitName: (name: string) => void;
}

const NameInputPage: React.FC<NameInputPageProps> = ({ onSubmitName }) => {
  const [name, setName] = useState("");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = name.trim();
    if (trimmed) {
      onSubmitName(trimmed);
    }
  };

  return (
    <div className="name-page-container">
      <div className="name-card">
        <h2 className="name-title">Digite o nome que deseja usar no chat:</h2>
        <form onSubmit={handleSubmit} className="name-form">
          <input
            type="text"
            className="name-input"
            placeholder="Seu nome aqui..."
            value={name}
            autoFocus
            onChange={(e) => setName(e.target.value)}
          />
          <button type="submit" className="name-button">
            Entrar no chat
          </button>
        </form>
      </div>
    </div>
  );
};

export default NameInputPage;
