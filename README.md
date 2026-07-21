# AgentWatch

Live-Monitor für aktive **Claude-Code-Sessions** unter Windows — zeigt Status, Anzahl der Agents/Subagents, Token-/Kontext-Auslastung und Rate-Limits in Echtzeit im System-Tray. Windows-Variante von [Irrlicht](https://github.com/ingo-eichhorst/Irrlicht).

## Features

- **Echtzeit-Status** je Session: 🔵 arbeitet · 🟠 wartet auf Eingabe · 🟢 bereit (Datei-Watcher, kein Polling). Ein laufender Shell-Befehl (`shell`-Status) zählt nur als „arbeitet", solange das Transcript aktiv beschrieben wird — ein zurückgelassener Hintergrund-Prozess (z. B. ein Dev-Server) färbt die Session so nicht dauerhaft als arbeitend.
- **Agent-/Subagent-Anzahl** je Session inkl. aufklappbarem Baum (Typ + Zustand), auch verschachtelte Workflow-Agents
- **Kontext-Auslastung** mit Pressure-Warnung (grün → orange → rot ab 80 %) gegen stille Auto-Compaction
- **Token-Anzeige** und optionale **Kostenschätzung (USD)** — kumulativ pro Session über alle Turns, mit getrennten Listenpreisen für Input/Output/Cache-Read/Cache-Write (offizielles Anthropic-Preisblatt)
- **Git-/Projekt-Gruppierung** der Sessions
- **History-Timeline** je Session (Zustandsverlauf)
- **Rate-Limit/Quota (5h/7d)** — opt-in via statusLine-Integration
- **Windows-Toast-Benachrichtigungen** (wartet / fertig / Context-Pressure / neuer Subagent) — **standardmäßig aus** (die Pill übernimmt die Signalisierung), einzeln in den Einstellungen aktivierbar, mit Sound-Schalter
- **Verankertes Dashboard-Flyout** statt frei stehendem Fenster: dasselbe volle Dashboard klappt **oben mittig direkt unter der Pill** (Pill-Klick) bzw. **unten rechts überm Tray** (Tray-Icon-Klick) auf. Es schließt automatisch bei Fokusverlust (Klick daneben), erneuter Klick toggelt, **Esc** schließt. **Tray-Icon** zeigt die dominante Statusfarbe.
- **„Dynamic Island"-Pill** oben mittig am Bildschirm (separates, transparentes Always-on-Top-Fenster): ein farbiger Punkt je Session, darunter die Anzahl der Subagents genau dieser Session. **Signalisiert Ereignisse visuell statt per Toast** — wartende Sessions pulsieren orange, gerade fertige blitzen grün, und bei einem Ereignis klappt die Pill kurz auf und zeigt in Stichpunkten, was passiert ist (z. B. „projects: fertig"). Monitor in den Einstellungen wählbar, Klick klappt das Dashboard direkt unter der Pill auf
- **Auto-Update**: prüft beim Start auf eine neue Version (GitHub-Releases-Manifest), zeigt bei Verfügbarkeit ein dezentes Banner im Dashboard — ein Klick lädt herunter, installiert und startet die App neu
- **Autostart** beim Windows-Login (konfigurierbar)
- **Local-first**: liest nur lokale Dateien, keine Telemetrie, fasst `.credentials.json` nie an

## Architektur

- **Backend (Rust / Tauri 2):** Datei-Watcher (`notify`) auf `~/.claude/sessions` + `~/.claude/projects`, Prozess-Liveness-Check (`sysinfo`), Transcript-Parser, Snapshot-Aggregator, Tray + Benachrichtigungen. Sendet Snapshots per Tauri-Event ans Frontend.
- **Frontend (React + TypeScript / Vite):** Dark-Dashboard, rendert die Snapshots.

### Datenquellen (alle read-only)

| Signal | Quelle |
|---|---|
| Live-Status (idle/busy/waiting) + Heartbeat | `~/.claude/sessions/<PID>.json` |
| Liveness-Check | laufende `claude`-Prozesse |
| Tokens, Modell, Git-Branch | `~/.claude/projects/<ordner>/<sessionId>.jsonl` |
| Subagents (Typ, Zustand) | `~/.claude/projects/<ordner>/<sessionId>/subagents/*.meta.json` |
| Rate-Limit (5h/7d) | opt-in: statusLine schreibt `~/.claude/agentwatch-statusline.json` |

### Modul-Übersicht (`src-tauri/src/`)

`model` (Datentypen) · `claude_paths` (Pfade/Encoding) · `sessions` · `processes` · `transcripts` · `subagents` · `pricing` · `aggregator` (Snapshot) · `history` (Timeline) · `watcher` (Echtzeit) · `notifications` · `statusline` (Rate-Limit opt-in) · `settings` · `tray` · `commands` · `lib` (Verdrahtung)

## Entwicklung

```bash
npm install
npm run tauri dev      # App + Vite-Dev-Server (Port 1420)
```

> Hinweis: `cargo` liegt unter `~/.cargo/bin` und ist evtl. nicht im PATH der Shell — ggf. voranstellen.

## Tests

```bash
npm test               # Frontend (Vitest)
cd src-tauri && cargo test --lib   # Backend (Rust)
```

## Build / Installer

```bash
npm run tauri build    # nativer Build + NSIS- & MSI-Installer
```

Artefakte unter `src-tauri/target/release/bundle/` (`nsis/`, `msi/`).

### Plattform-Hinweis (ARM64 / x64)

Der Build erfolgt für die Host-Architektur. Für einen x64-Installer (Verteilung an Kollegen):

```bash
rustup target add x86_64-pc-windows-msvc
npm run tauri build -- --target x86_64-pc-windows-msvc
```

Voraussetzung Build: Rust + VS-2022-C++-Build-Tools (für ARM64 zusätzlich die Komponente *MSVC v143 – C++ ARM64/ARM64EC-Buildtools, neueste Version*), WebView2-Runtime.

### Auto-Update / Releases

Lokale `npm run tauri build`-Läufe erzeugen normale, unsignierte Installer (kein Updater-Artefakt nötig). Die CI (`.github/workflows/build-windows.yml`) baut zusätzlich mit `--config '{"bundle":{"createUpdaterArtifacts":true}}'` und signiert dabei mit den Repo-Secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (Ed25519-Keypair via `npx tauri signer generate`, unabhängig von Code-Signing/Authenticode). Der `release`-Job baut daraus ein `latest.json`-Manifest und hängt es an den GitHub Release — das ist der Endpoint, den die App zur Update-Prüfung abfragt (`plugins.updater.endpoints` in `tauri.conf.json`).

## Lizenz / Hersteller

v1: unsignierte Installer (Code-Signing folgt). Update-Pakete sind ab v0.2.0 mit einem eigenen Ed25519-Key signiert (Tauri-Updater), unabhängig vom offenen Punkt Authenticode/Code-Signing.
