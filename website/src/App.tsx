import React, { useState, useEffect } from "react";
import LoadingScreen from "./pages/loadingPage";
import NameInputPage from "./pages/nameInputPage";
import ChatScreen from "./pages/chatPage";

const App: React.FC = () => {
  const [loading, setLoading] = useState(true);
  const [userName, setUserName] = useState<string | null>(null);

  useEffect(() => {
    const timer = setTimeout(() => setLoading(false), 3000);
    return () => clearTimeout(timer);
  }, []);

  if (loading) {
    return <LoadingScreen />;
  }

  if (!userName) {
    return <NameInputPage onSubmitName={(name) => setUserName(name)} />;
  }

  return <ChatScreen userName={userName} />;
};

export default App;
