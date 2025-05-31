import React from "react";
import "../assets/css/loadingPage.css";
import logo from "../assets/Logo2.png";

const LoadingScreen: React.FC = () => (
  <div className="loading-container">
    {/* Logo estática e maior, posicionada no topo */}
    <img src={logo} alt="Logo" className="large-logo" />

    {/* Spinner (bolinha girando) abaixo da logo */}
    <div className="spinner" />
  </div>
);

export default LoadingScreen;
