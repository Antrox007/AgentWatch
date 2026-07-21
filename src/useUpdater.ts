// Prueft auf eine neue Version (Tauri-Updater-Plugin) und bietet Download +
// Installation + Neustart per einzelnem Klick an. Zwei Ausloeser: automatisch
// beim Start (still, sofern in den Einstellungen aktiviert) und manuell per
// "Jetzt pruefen"-Button in den Einstellungen (immer mit Rueckmeldung, auch
// wenn kein Update gefunden wurde oder die Pruefung fehlschlaegt).

import { useCallback, useEffect, useRef, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdatePhase =
  | "idle"
  | "checking"
  | "up-to-date"
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

// Wie lange die "Du bist aktuell"-Rueckmeldung nach einer manuellen Pruefung
// stehen bleibt, bevor sie wieder verschwindet.
const UP_TO_DATE_FLASH_MS = 2500;

export function useUpdater(autoCheck: boolean) {
  const [state, setState] = useState<UpdaterState>(INITIAL_STATE);
  const updateRef = useRef<Update | null>(null);

  // silent=true (Auto-Check beim Start): bei "kein Update"/Fehler einfach
  // still bleiben, statt den Nutzer mit einer Meldung zu behelligen, die er
  // nicht angefordert hat. silent=false (Button-Klick): immer eine
  // Rueckmeldung geben, da eine explizite Aktion sonst wirkungslos wirkt.
  const runCheck = useCallback(async (opts: { silent: boolean }) => {
    setState((s) => ({ ...s, phase: "checking", error: null }));
    try {
      const update = await check();
      if (update) {
        updateRef.current = update;
        setState({
          phase: "available",
          version: update.version,
          progressPct: null,
          error: null,
        });
      } else if (opts.silent) {
        setState(INITIAL_STATE);
      } else {
        setState({ phase: "up-to-date", version: null, progressPct: null, error: null });
        setTimeout(() => {
          setState((s) => (s.phase === "up-to-date" ? INITIAL_STATE : s));
        }, UP_TO_DATE_FLASH_MS);
      }
    } catch (err) {
      if (opts.silent) {
        setState(INITIAL_STATE);
      } else {
        setState({ phase: "error", version: null, progressPct: null, error: String(err) });
      }
    }
  }, []);

  useEffect(() => {
    if (!autoCheck) return;
    runCheck({ silent: true });
  }, [autoCheck, runCheck]);

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

  const checkNow = useCallback(() => {
    runCheck({ silent: false });
  }, [runCheck]);

  return { state, install, dismiss, checkNow };
}
