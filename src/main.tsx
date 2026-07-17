import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import App from "./App";
import "./index.css";
import { applyTheme, getStoredTheme } from "@/lib/theme";

// 首帧前应用主题，避免亮暗闪烁
applyTheme(getStoredTheme());

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
