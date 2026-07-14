# OPEN_ISSUES — AgentWatch

Offene Punkte und bewusst zurückgestellte Entscheidungen. Stand: 2026-06-17.

## Offen

- **Code-Signing** — v1 liefert **unsignierte** Installer (NSIS/MSI). Sobald ein Code-Signing-Zertifikat vorliegt, `signtool` in den Build einbinden.
- **Native ARM64- und x64-Build** — beide funktionieren und sind **aktuell (2026-06-17)**, inkl. der Pill-Umgestaltung (Agent-Zahl je Session, Statuszähler entfernt, Flamme entfernt). ARM64 via `npm run tauri build`, x64 via `npm run tauri build -- --target x86_64-pc-windows-msvc` (beide Targets installiert). Der x64-Build läuft auf dem Snapdragon zusätzlich via Prism-Emulation. Artefakte: `target/release/bundle/` (ARM64) bzw. `target/x86_64-pc-windows-msvc/release/bundle/` (x64), jeweils `nsis/` + `msi/`.
- **Manueller End-to-End-Test** der Installer auf einem echten x64-Kollegen-Rechner steht aus.

## Bekannte Einschränkungen (akzeptiert)

- **Kostenschätzung** ist informativ (Subscription) — Listenpreise pro Token-Eimer, keine exakte Abrechnung.
- **Kontextfenster** wird aus der Modellfamilie abgeleitet. Läuft eine Claude-Code-Session auf einem 1M-fähigen Modell **ohne** aktivierten 1M-Modus (Claude Code deckelt dann früher), zeigt AgentWatch das Fenster etwas zu großzügig — aus dem Transcript nicht unterscheidbar.
- **Status-Heuristik:** ein einzelner langer Vordergrund-Befehl (> 30 s ohne Transcript-Aktivität) erscheint kurz als „bereit", bis wieder Aktivität kommt (Tradeoff gegen dauerhaft als „arbeitet" angezeigte zurückgelassene Hintergrund-Shells).

## Bewusst zurückgestellt (LOW)

- History-Clone pro Tick, rekursiver notify-Watch (Debounce fängt es ab), utilization-Clamp.
- Animiertes „Aufklapp"-Easing des Flyouts über Fenstergrenzen hinweg (OS-Fenster nur begrenzt smooth animierbar) — das Flyout erscheint/verschwindet direkt. Optional später eine kurze CSS-Einblendung im Dashboard-Content.

## Geändert (2026-06-17)

- **Pill umgestaltet (Agent-Anzahl statt Statuszähler):** Die „Dynamic Island"-Pill zeigt jetzt unter jedem Statuspunkt die **Anzahl der Subagents genau dieser Session** (aus dem vorhandenen `subagentCount`; `0` gedimmt, `>0` hervorgehoben). Die bisherigen **rechten Statuszähler** (working/waiting/ready, `.island-counts`) wurden **entfernt** — die Anzahl je Status ist durch die Punkte ohnehin abzählbar. Außerdem die **🔥-Flamme** (`.island-glyph` + `flameFlicker`-Animation) komplett aus der Pill entfernt. Betrifft `src/Island.tsx` + `src/App.css` (inkl. zugehöriger CSS-Regeln, `islandCountPop`-Animation und Reduced-Motion-Verweise). tsc sauber, 9/9 Vitest grün, nativer ARM64-Build neu erzeugt. Das **Dashboard** (Header-Logo + Leer-Zustand) behält seine Flamme bewusst.

## Geändert (2026-06-16)

- **Pill konnte auf veraltetem Snapshot einfrieren (Drift-Bug):** Das randlose, transparente, nie fokussierte Always-on-Top-Pill-Fenster wird von WebView2 im Hintergrund gedrosselt — gepushte `snapshot`-Events kamen zeitweise nicht mehr an, und die Pill hatte keinen Fallback zum Nachladen. Folge: Sie zeigte längst beendete Sessions (z. B. 9 „wartet"-Punkte), während das Dashboard korrekt 2 lebende Sessions zeigte. Fix in `src/Island.tsx`: zusätzlicher **Heartbeat-Poll** (`getSnapshot` per `invoke` alle 3 s) als Selbstheilung, unabhängig von den gedrosselten Events. Außerdem **doppelter React-`key`** in `.island-counts` behoben (Wert als Key kollidierte bei gleichem Zählerwert in zwei Slots, z. B. 1 arbeitet + 1 bereit → jetzt Slot-Präfix `w`/`a`/`r`).
- **Pill blockiert keine Hintergrundfenster mehr:** Das Pill-Fenster (Rechteck inkl. transparentem Bleed-Rand) fing bisher Mausklicks über seine gesamte Fläche ab — auch im transparenten Bereich über/neben der sichtbaren Pill, also genau in der Titelleisten-Zone am oberen Bildschirmrand. Jetzt ist das Fenster standardmäßig klick-durchlässig (`ignore_cursor_events`); ein Hintergrund-Thread (`island::start_cursor_passthrough`, 60-ms-Poll) schaltet das Abfangen nur ein, solange der globale Cursor über dem Pill-Rechteck (Fenster minus Bleed) liegt. Polling statt Webview-Mausevents, weil das Webview bei aktivem `ignore_cursor_events(true)` keine Mausnachrichten mehr erhält und das Wieder-Betreten nicht erkennen könnte. Glow/Shadow/Hover bleiben sichtbar (Rendering wird nicht beschnitten).

## Geändert (2026-06-11)

- **Frei stehendes Hauptfenster entfernt:** Das `main`-Fenster ist jetzt ein **verankertes Flyout** (start `visible:false`, `center:false`, `alwaysOnTop:true`, `resizable:false`, Drag-Region raus). Pill-Klick → oben mittig unter der Pill; Tray-Klick → unten rechts überm Tray; Tray-Menü „Fenster anzeigen" → unten rechts der Monitor-Fläche.
- **Hide-on-Blur ist jetzt AN** (vorher bewusst aus): das Flyout schließt bei Fokusverlust. Erneuter Pill-/Tray-Klick toggelt, **Esc** schließt. Die Blur↔Klick-Race ist über einen 300-ms-Guard (`DashboardGuard`) entschärft.
