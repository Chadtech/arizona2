use chrono::{DateTime, TimeZone, Utc};
use std::sync::OnceLock;

// Temporary cleanup boundary for reaction/event context after history contamination.
// Remove this module and its call sites once the system has stabilized.
pub fn event_history_cutoff() -> DateTime<Utc> {
    static CUTOFF: OnceLock<DateTime<Utc>> = OnceLock::new();

    *CUTOFF.get_or_init(build_event_history_cutoff)
}

fn build_event_history_cutoff() -> DateTime<Utc> {
    match Utc.with_ymd_and_hms(2026, 3, 18, 0, 0, 0).single() {
        Some(cutoff) => cutoff,
        None => Utc::now(),
    }
}
