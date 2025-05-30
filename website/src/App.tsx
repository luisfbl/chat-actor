// App.tsx
import React, { useState, useEffect } from "react";
import LoadingScreen from "./pages/loadingPage";
import ChatScreen from "./pages/chatPage";

const App: React.FC = () => {
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Simula 3 segundos de carregamento
    const timer = setTimeout(() => setLoading(false), 3000);
    return () => clearTimeout(timer);
  }, []);

  return <>{loading ? <LoadingScreen /> : <ChatScreen />}</>;
};

export default App;
