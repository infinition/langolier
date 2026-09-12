import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Palette from "./pages/Palette";
import { LangProvider } from "./i18n";
import "./style.css";

// The floating window loads the same app behind the #palette anchor
const palette = window.location.hash === "#palette";
if (palette) document.documentElement.classList.add("palette-mode");
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <LangProvider>
      {(lang) => (palette ? <Palette key={lang} /> : <App key={lang} />)}
    </LangProvider>
  </React.StrictMode>,
);
