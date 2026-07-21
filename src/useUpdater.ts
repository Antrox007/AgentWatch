// Prueft beim Start einmal auf eine neue Version (Tauri-Updater-Plugin) und
// bietet Download + Installation + Neustart per einzelnem Klick an.

import { useCallback, useEffect, useRef, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdatePhase =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "restarting"
  | "error";

export interface UpdaterState {
  phase: UpdatePhase;
  version: string | null;
  progressPct: number | null;
  error: string | null;
}

const INITIAL_STATE: UpdaterState = {
  phase: "idle",
  version: null,
  progressPct: null,
  error: null,
};

export function useUpdater() {
  const [state, setState] = useState<UpdaterState>(INITIAL_STATE);
  const updateRef = useRef<Update | null>(null);

  useEffect(() => {
    let cancelled = false;
    setState((s) => ({ ...s, phase: "checking" }));
    check()
      .then((update) => {
        if (cancelled) return;
        if (update) {
          updateRef.current = update;
          setState({
            phase: "available",
            version: update.version,
            progressPct: null,
            error: null,
          });
        } else {
          setState(INITIAL_STATE);
        }
      })
      .catch(() => {
        // Kein Netz, Endpoint nicht erreichbar, oder reine Browser-Vorschau
        // ohne Tauri-Runtime — Update-Check ist rein informativ, still bleiben.
        if (!cancelled) setState(INITIAL_STATE);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const install = useCallback(async () => {
    const update = updateRef.current;
    if (!update) return;

    let contentLength = 0;
    let received = 0;
    try {
      setState((s) => ({ ...s, phase: "downloading", progressPct: 0, error: null }));
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            contentLength = event.data.contentLength ?? 0;
            break;
          case "Progress":
            received += event.data.chunkLength;
            setState((s) => ({
              ...s,
              progressPct:
                contentLength > 0
                  ? Math.min(100, Math.round((received / contentLength) * 100))
                  : s.progressPct,
            }));
            break;
          case "Finished":
            setState((s) => ({ ...s, progressPct: 100 }));
            break;
        }
      });
      setState((s) => ({ ...s, phase: "restarting" }));
      await relaunch();
    } catch (err) {
      setState((s) => ({ ...s, phase: "error", error: String(err) }));
    }
  }, []);

  const dismiss = useCallback(() => {
    setState(INITIAL_STATE);
  }, []);

  return { state, install, dismiss };
}
