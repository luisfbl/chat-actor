import React from "react";
import "../assets/css/loadingPage.css";
import logo from "../assets/Logo2.png";

const LoadingScreen: React.FC = () => (
  <div className="loading-container">
    <img src={logo} alt="Sad Discord Logo" className="loading-logo" />
  </div>
);

export default LoadingScreen;
