import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import Island from "./Island";

/**
 * Erkennt, ob dieses Fenster die "Dynamic Island"-Pill ist. Das Backend oeffnet
 * das Island-Fenster mit der URL `index.html?island`; zusaetzlich pruefen wir das
 * Tauri-Fensterlabel als Fallback (in der reinen Browser-Vorschau immer false).
 */
function isIslandView(): boolean {
  if (
    window.location.search.includes("island") ||
    window.location.hash.includes("island")
  ) {
    return true;
  }
  try {
    const internals = (window as unknown as {
      __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } };
    }).__TAURI_INTERNALS__;
    return internals?.metadata?.currentWindow?.label === "island";
  } catch {
    return false;
  }
}

const island = isIslandView();
if (island) {
  document.body.classList.add("island-body");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>{island ? <Island /> : <App />}</React.StrictMode>,
);
