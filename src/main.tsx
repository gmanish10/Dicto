import React from "react";
import ReactDOM from "react-dom/client";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import App from "./App";
import Onboarding from "./routes/Onboarding";
import Settings from "./routes/Settings";
import Dictionary from "./routes/Dictionary";
import History from "./routes/History";
import About from "./routes/About";
import "./styles/index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <HashRouter>
      <Routes>
        <Route path="/onboarding" element={<Onboarding />} />
        <Route element={<App />}>
          <Route path="/" element={<Navigate to="/settings" replace />} />
          <Route path="/settings" element={<Settings />} />
          <Route path="/dictionary" element={<Dictionary />} />
          <Route path="/history" element={<History />} />
          <Route path="/about" element={<About />} />
        </Route>
      </Routes>
    </HashRouter>
  </React.StrictMode>
);
