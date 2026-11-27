import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./design/tokens.css";
import { App } from "./App";
import { installErrorReporting } from "./errorReporting";

installErrorReporting();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
