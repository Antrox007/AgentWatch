//! Hält pro Session eine rollierende Zustands-Historie und reichert den
//! Snapshot damit an. Lebt im Watcher-Thread (ueber Ticks hinweg persistent).

use crate::model::{HistoryPoint, Snapshot};
use std::collections::{HashMap, HashSet};

/// Maximale Anzahl Historien-Punkte pro Session (~ letzte 3 Min bei 2s-Tick).
const MAX_POINTS: usize = 90;

#[derive(Default)]
pub struct HistoryStore {
    map: HashMap<String, Vec<HistoryPoint>>,
}

impl HistoryStore {
    /// Haengt fuer jede Session den aktuellen Zustand an und schreibt die
    /// Historie zurueck in den Snapshot. Verschwundene Sessions werden vergessen.
    pub fn record_and_enrich(&mut self, snapshot: &mut Snapshot) {
        let now = snapshot.generated_at;
        let mut seen = HashSet::new();

        for group in &mut snapshot.projects {
            for session in &mut group.sessions {
                seen.insert(session.session_id.clone());
                let entry = self.map.entry(session.session_id.clone()).or_default();
                entry.push(HistoryPoint {
                    t: now,
                    state: session.state,
                });
                if entry.len() > MAX_POINTS {
                    let excess = entry.len() - MAX_POINTS;
                    entry.drain(0..excess);
                }
                session.history = entry.clone();
            }
        }

        self.map.retain(|id, _| seen.contains(id));
    }
}
