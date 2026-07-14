//! Aufloesung der Claude-Code-Pfade auf dem lokalen System.

use std::path::PathBuf;

/// Basisverzeichnis `~/.claude` (bzw. `CLAUDE_CONFIG_DIR`, falls absolut gesetzt).
pub fn claude_dir() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("CLAUDE_CONFIG_DIR") {
        let p = PathBuf::from(&custom);
        if p.is_absolute() {
            return Some(p);
        }
    }
    dirs::home_dir().map(|h| h.join(".claude"))
}

/// Verzeichnis mit den Live-Session-Markern `sessions/<PID>.json`.
pub fn sessions_dir() -> Option<PathBuf> {
    claude_dir().map(|c| c.join("sessions"))
}

/// Verzeichnis mit den Transcript-Ordnern `projects/<projektordner>/...`.
pub fn projects_dir() -> Option<PathBuf> {
    claude_dir().map(|c| c.join("projects"))
}

/// Wandelt ein Arbeitsverzeichnis in den Claude-Projektordnernamen um.
/// Claude Code ersetzt JEDES nicht-alphanumerische Zeichen (Doppelpunkt,
/// Backslash, Slash, Punkt, Leerzeichen, Bindestrich, ...) durch `-`.
/// Beispiele:
///   `C:\projects\todo-app` -> `C--projects-todo-app`
///   `C:\Users\user\OneDrive - Acme` -> `C--Users-user-OneDrive---Acme`
pub fn encode_project_folder(cwd: &str) -> String {
    cwd.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect()
}

/// Letztes Pfadsegment als lesbarer Projektname.
pub fn project_name_from_cwd(cwd: &str) -> String {
    let trimmed = cwd.trim_end_matches(['\\', '/']);
    let name = trimmed.rsplit(['\\', '/']).next().unwrap_or(trimmed);
    if name.is_empty() {
        cwd.to_string()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_windows_path() {
        // C:\projects\todo-app -> C--projects-todo-app
        assert_eq!(
            encode_project_folder("C:\\projects\\todo-app"),
            "C--projects-todo-app"
        );
    }

    #[test]
    fn encode_forward_slashes() {
        assert_eq!(encode_project_folder("D:/code/app"), "D--code-app");
    }

    #[test]
    fn encode_dots_spaces_dashes() {
        // Punkte, Leerzeichen und Bindestriche werden alle zu '-'.
        assert_eq!(
            encode_project_folder("C:\\Users\\user\\OneDrive - Acme"),
            "C--Users-user-OneDrive---Acme"
        );
    }

    #[test]
    fn project_name_basic() {
        assert_eq!(project_name_from_cwd("C:\\projects\\demo-project"), "demo-project");
        assert_eq!(
            project_name_from_cwd("C:\\projects\\api-server\\"),
            "api-server"
        );
        assert_eq!(project_name_from_cwd("/home/user/app"), "app");
    }
}
