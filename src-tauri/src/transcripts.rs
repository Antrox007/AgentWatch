//! Liest aus dem Transcript einer Session (`projects/<ordner>/<sessionId>.jsonl`)
//! Modell, aktuelle Kontextgroesse, Git-Branch, letzten Assistant-Text und die
//! kumulativen USD-Kosten der Session (Summe ueber alle Assistant-Turns).
//!
//! Transcripts sind append-only. Der Parser liest die Datei daher inkrementell:
//! pro Pfad wird gemerkt, bis zu welchem Byte-Offset bereits geparst wurde, und
//! beim naechsten Mal nur der angehaengte Rest gelesen. Unveraenderte Transcripts
//! (gleiche mtime) werden gar nicht erneut angefasst.

use crate::claude_paths;
use crate::pricing::{self, TokenUsage};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

#[derive(Debug, Default, Clone)]
pub struct TranscriptInfo {
    pub model: Option<String>,
    pub context_tokens: Option<u64>,
    pub git_branch: Option<String>,
    pub last_text: Option<String>,
    /// Kumulative geschaetzte Kosten der Session in USD.
    pub estimated_cost_usd: Option<f64>,
}

/// Fortlaufend akkumulierter Zustand pro Transcript (ueber alle bisher geparsten Turns).
#[derive(Debug, Default, Clone)]
struct Accum {
    cost_usd: f64,
    saw_assistant: bool,
    model: Option<String>,
    git_branch: Option<String>,
    last_text: Option<String>,
    context_tokens: Option<u64>,
}

impl Accum {
    fn to_info(&self) -> TranscriptInfo {
        TranscriptInfo {
            model: self.model.clone(),
            context_tokens: self.context_tokens,
            git_branch: self.git_branch.clone(),
            last_text: self.last_text.clone(),
            estimated_cost_usd: self.saw_assistant.then_some(self.cost_usd),
        }
    }
}

/// Cache-Eintrag pro Transcript-Pfad: bis wohin geparst, was bisher gesehen wurde.
#[derive(Debug, Default, Clone)]
struct CacheEntry {
    mtime: Option<SystemTime>,
    parsed_len: u64,
    seen_ids: HashSet<String>,
    acc: Accum,
}

/// Cache: Pfad -> inkrementeller Parse-Zustand.
static CACHE: LazyLock<Mutex<HashMap<PathBuf, CacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Findet die Transcript-Datei zu einer Session.
pub fn find_transcript(cwd: &str, session_id: &str) -> Option<PathBuf> {
    let projects = claude_paths::projects_dir()?;
    let folder = claude_paths::encode_project_folder(cwd);
    let direct = projects.join(&folder).join(format!("{session_id}.jsonl"));
    if direct.is_file() {
        return Some(direct);
    }
    // Fallback: alle Projektordner nach passender Datei durchsuchen.
    if let Ok(entries) = std::fs::read_dir(&projects) {
        for entry in entries.flatten() {
            let cand = entry.path().join(format!("{session_id}.jsonl"));
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

/// Liefert die Transcript-Infos — inkrementell geparst und nach mtime gecached.
pub fn read_transcript_info(path: &Path) -> TranscriptInfo {
    let meta = std::fs::metadata(path);
    let mtime = meta.as_ref().ok().and_then(|m| m.modified().ok());
    let len = meta.as_ref().map(|m| m.len()).unwrap_or(0);

    // Vorherigen Zustand holen (oder frisch beginnen). Lock nur kurz halten.
    let prev = {
        let cache = CACHE.lock().ok();
        cache.as_ref().and_then(|c| c.get(path).cloned())
    };

    if let Some(entry) = &prev {
        // Unveraendert (gleiche mtime) -> direkt aus dem Akkumulator liefern.
        if entry.mtime == mtime && mtime.is_some() {
            return entry.acc.to_info();
        }
    }

    // Start-Offset bestimmen: anhaengen, sofern die Datei nur gewachsen ist.
    let (start, mut acc, mut seen) = match &prev {
        Some(entry) if len >= entry.parsed_len => {
            (entry.parsed_len, entry.acc.clone(), entry.seen_ids.clone())
        }
        // Neu oder Datei geschrumpft/ersetzt -> von vorne parsen.
        _ => (0, Accum::default(), HashSet::new()),
    };

    let chunk = read_from(path, start).unwrap_or_default();
    let consumed = parse_chunk(&chunk, &mut acc, &mut seen);
    let parsed_len = start + consumed as u64;

    let info = acc.to_info();
    if let Ok(mut cache) = CACHE.lock() {
        cache.insert(
            path.to_path_buf(),
            CacheEntry {
                mtime,
                parsed_len,
                seen_ids: seen,
                acc,
            },
        );
    }
    info
}

/// Liest eine Datei ab `offset` bis zum Ende als verlustfreie UTF-8-Zeichenkette.
fn read_from(path: &Path, offset: u64) -> Option<String> {
    let mut file = File::open(path).ok()?;
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// Parst alle vollstaendigen Zeilen (bis zum letzten `\n`) aus `chunk` in den
/// Akkumulator und gibt zurueck, wie viele Bytes verarbeitet wurden. Eine
/// abschliessende, noch unvollstaendige Zeile bleibt fuer den naechsten Lauf liegen.
fn parse_chunk(chunk: &str, acc: &mut Accum, seen: &mut HashSet<String>) -> usize {
    let Some(last_nl) = chunk.rfind('\n') else {
        return 0;
    };
    let complete = &chunk[..=last_nl];

    for line in complete.lines() {
        let line = line.trim();
        if !line.starts_with('{') {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        apply_line(&value, acc, seen);
    }
    last_nl + 1
}

/// Verarbeitet eine einzelne JSONL-Zeile: aktualisiert Branch und (bei
/// Assistant-Nachrichten) Modell, Kontextgroesse, letzten Text und Kosten.
fn apply_line(value: &Value, acc: &mut Accum, seen: &mut HashSet<String>) {
    if let Some(branch) = value.get("gitBranch").and_then(Value::as_str) {
        if !branch.is_empty() {
            acc.git_branch = Some(branch.to_string());
        }
    }

    if value.get("type").and_then(Value::as_str) != Some("assistant") {
        return;
    }
    let Some(message) = value.get("message") else {
        return;
    };
    acc.saw_assistant = true;

    let turn_model = message
        .get("model")
        .and_then(Value::as_str)
        .map(str::to_string);
    if let Some(m) = &turn_model {
        acc.model = Some(m.clone());
    }

    if let Some(usage) = message.get("usage") {
        let (tokens, context) = parse_usage(usage);
        if context > 0 {
            acc.context_tokens = Some(context);
        }
        // Kosten nur einmal pro Nachricht zaehlen (Dedup ueber message.id).
        let id = message.get("id").and_then(Value::as_str);
        let already = id.map(|i| seen.contains(i)).unwrap_or(false);
        if !already {
            if let Some(i) = id {
                seen.insert(i.to_string());
            }
            // Preis nach dem Modell dieses Turns (Fallback: zuletzt bekanntes).
            let model = turn_model
                .as_deref()
                .or(acc.model.as_deref())
                .unwrap_or("");
            acc.cost_usd += pricing::turn_cost_usd(model, &tokens);
        }
    }

    if let Some(content) = message.get("content").and_then(Value::as_array) {
        for block in content {
            if block.get("type").and_then(Value::as_str) == Some("text") {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    let text = text.trim();
                    if !text.is_empty() {
                        acc.last_text = Some(text.chars().take(240).collect());
                    }
                }
            }
        }
    }
}

/// Zerlegt das `usage`-Objekt in die Preis-Eimer und die kombinierte Kontextgroesse.
fn parse_usage(usage: &Value) -> (TokenUsage, u64) {
    let field = |k: &str| usage.get(k).and_then(Value::as_u64).unwrap_or(0);
    let input = field("input_tokens");
    let output = field("output_tokens");
    let cache_read = field("cache_read_input_tokens");
    let cache_creation_total = field("cache_creation_input_tokens");

    // Cache-Writes nach TTL aufteilen, falls die Detailfelder vorhanden sind.
    let (w5, w1) = match usage.get("cache_creation").and_then(Value::as_object) {
        Some(cc) => {
            let g = |k: &str| cc.get(k).and_then(Value::as_u64).unwrap_or(0);
            (
                g("ephemeral_5m_input_tokens"),
                g("ephemeral_1h_input_tokens"),
            )
        }
        None => (0, 0),
    };
    // Ohne Detailfelder: gesamten Cache-Write als 5-Min-TTL behandeln (Default).
    let (cache_write_5m, cache_write_1h) = if w5 + w1 > 0 {
        (w5, w1)
    } else {
        (cache_creation_total, 0)
    };

    // Kontextgroesse = Input + Cache (+ Output des letzten Turns, der im
    // naechsten Turn Teil des Inputs wird).
    let context = input + cache_read + cache_creation_total + output;

    (
        TokenUsage {
            input,
            output,
            cache_read,
            cache_write_5m,
            cache_write_1h,
        },
        context,
    )
}

/// Letzte Aenderung der Transcript-Datei in Epoch-Millisekunden (UTC).
/// Dient als Aktivitaets-Signal: waehrend echter Arbeit waechst das Transcript
/// (Nachrichten, Tool-Aufrufe), im Leerlauf bleibt es unveraendert.
pub fn file_mtime_millis(path: &Path) -> Option<i64> {
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok()?;
    let dur = modified.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(dur.as_millis() as i64)
}

/// Liefert das native Kontextfenster (Token) fuer ein Modell — gemaess
/// offiziellem Anthropic-Preisblatt. Der Claude-Code-Marker `[1m]` (bzw. `-1m`)
/// hat Vorrang, falls vorhanden; ansonsten wird das Fenster anhand der
/// Modellfamilie bestimmt. Hinweis: Im Transcript steht nur die nackte
/// Modell-ID (z.B. `claude-opus-4-8`), nicht das Claude-Code-`[1m]`-Suffix.
pub fn context_window_for(model: &str) -> u64 {
    let m = model.to_lowercase();
    // Expliziter 1M-Marker (Claude-Code-Suffix) hat Vorrang.
    if m.contains("[1m]") || m.contains("-1m") || m.contains(" 1m") {
        return 1_000_000;
    }
    if m.contains("haiku") {
        // Haiku 4.5: 200K. (Aeltere Haikus ebenfalls <= 200K.)
        200_000
    } else if m.contains("opus") {
        // Opus 4.5/4.6/4.7/4.8: 1M nativ (Opus 3 ist abgekuendigt).
        1_000_000
    } else if m.contains("fable") {
        1_000_000
    } else if m.contains("sonnet-4-6") || m.contains("sonnet-4.6") {
        // Sonnet 4.6: 1M nativ.
        1_000_000
    } else if m.contains("sonnet") {
        // Aeltere Sonnets (4.5/4/3.x): 200K Standardfenster.
        200_000
    } else {
        // Unbekannt -> konservativ 200K.
        200_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_window_per_model_family() {
        // Opus 4.5+ und Sonnet 4.6 sind nativ 1M.
        assert_eq!(context_window_for("claude-opus-4-8"), 1_000_000);
        assert_eq!(context_window_for("claude-opus-4-8[1m]"), 1_000_000);
        assert_eq!(context_window_for("claude-sonnet-4-6"), 1_000_000);
        assert_eq!(context_window_for("claude-fable-5"), 1_000_000);
        // Haiku und aeltere Sonnets: 200K.
        assert_eq!(context_window_for("claude-haiku-4-5"), 200_000);
        assert_eq!(context_window_for("claude-sonnet-4-5"), 200_000);
        // Expliziter 1M-Marker ueberschreibt das Familien-Default.
        assert_eq!(context_window_for("claude-sonnet-4-5-1m"), 1_000_000);
    }

    #[test]
    fn accumulates_cost_across_turns_and_dedupes() {
        let mut acc = Accum::default();
        let mut seen = HashSet::new();

        // Zwei Assistant-Turns (Opus): je 1 Mio Output = 25 USD -> 50 USD gesamt.
        let turn = |id: &str| {
            format!(
                r#"{{"type":"assistant","message":{{"id":"{id}","model":"claude-opus-4-8","usage":{{"output_tokens":1000000}}}}}}"#
            )
        };
        let chunk = format!("{}\n{}\n", turn("msg_a"), turn("msg_b"));
        parse_chunk(&chunk, &mut acc, &mut seen);
        assert!((acc.cost_usd - 50.0).abs() < 1e-6);

        // Gleiche message.id erneut -> wird nicht doppelt gezaehlt.
        let dup = format!("{}\n", turn("msg_a"));
        parse_chunk(&dup, &mut acc, &mut seen);
        assert!((acc.cost_usd - 50.0).abs() < 1e-6);
    }

    #[test]
    fn parse_chunk_leaves_incomplete_trailing_line() {
        let mut acc = Accum::default();
        let mut seen = HashSet::new();
        let chunk = "{\"type\":\"x\"}\n{\"type\":\"incompl";
        let consumed = parse_chunk(chunk, &mut acc, &mut seen);
        // Nur bis einschliesslich des letzten Zeilenumbruchs.
        assert_eq!(consumed, "{\"type\":\"x\"}\n".len());
    }

    #[test]
    fn splits_cache_write_by_ttl() {
        let usage: Value = serde_json::from_str(
            r#"{"cache_creation_input_tokens":300,"cache_creation":{"ephemeral_5m_input_tokens":200,"ephemeral_1h_input_tokens":100}}"#,
        )
        .unwrap();
        let (tokens, _) = parse_usage(&usage);
        assert_eq!(tokens.cache_write_5m, 200);
        assert_eq!(tokens.cache_write_1h, 100);
    }
}
