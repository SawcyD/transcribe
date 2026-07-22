import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { FluentProvider } from "@memora/ui";
import "@memora/ui/styles.css";
import App from "./App";
import "./styles/global.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <FluentProvider theme="system" density="compact">
      <App />
    </FluentProvider>
  </StrictMode>,
);
