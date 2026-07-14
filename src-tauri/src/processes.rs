//! Prozess-Scan zur Liveness-Verifikation der Sessions.
//! Ein Session-Marker gilt nur als aktiv, wenn ein laufender `claude`-Prozess
//! mit passender PID existiert (verhindert verwaiste Marker).

use std::collections::HashSet;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

/// PIDs aller laufenden Prozesse, deren Name "claude" enthaelt.
///
/// Es wird bewusst NUR die Prozessliste (ohne CPU/RAM/Disk/Netz und ohne
/// Detail-Daten wie cmd/exe/cwd) aktualisiert — gebraucht werden nur Name + PID.
pub fn alive_claude_pids() -> HashSet<u32> {
    let mut sys = System::new();
    // ProcessRefreshKind::new() = nichts extra (kein cpu/mem/disk/cmd) — nur Name+PID.
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::new());

    let mut set = HashSet::new();
    for (pid, process) in sys.processes() {
        if process
            .name()
            .to_string_lossy()
            .to_lowercase()
            .contains("claude")
        {
            set.insert(pid.as_u32());
        }
    }
    set
}
