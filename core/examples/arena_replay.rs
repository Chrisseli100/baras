//! Throwaway validation: replay PvP logs and print encounter boundaries.
//! Usage: cargo run -p baras-core --example arena_replay -- <log> [<log>...]

use std::io::Read;

use baras_core::combat_log::LogParser;
use baras_core::signal_processor::{EventProcessor, GameSignal};
use baras_core::state::SessionCache;

fn main() {
    for path in std::env::args().skip(1) {
        println!("=== {path}");
        let mut bytes = Vec::new();
        std::fs::File::open(&path)
            .expect("open log")
            .read_to_end(&mut bytes)
            .expect("read log");
        let content = String::from_utf8_lossy(&bytes);

        let parser = LogParser::new(chrono::Local::now().naive_local());
        let mut processor = EventProcessor::new();
        let mut cache = SessionCache::default();

        let mut start: Option<chrono::NaiveDateTime> = None;
        for (line_num, line) in content.lines().enumerate() {
            if let Some(event) = parser.parse_line(line_num as u64, line) {
                let (signals, _event, _) = processor.process_event(event, &mut cache);
                for s in signals {
                    match s {
                        GameSignal::AreaEntered {
                            timestamp,
                            area_name,
                            ..
                        } => println!("  {timestamp} AREA {area_name}"),
                        GameSignal::CombatStarted { timestamp, .. } => start = Some(timestamp),
                        GameSignal::CombatEnded {
                            timestamp, success, ..
                        } => {
                            let dur = start
                                .map(|st| (timestamp - st).num_seconds())
                                .unwrap_or(0);
                            let name = cache
                                .last_combat_encounter()
                                .and_then(|e| e.area_name.clone())
                                .unwrap_or_default();
                            println!(
                                "  {} .. {timestamp} ({dur:>4}s) success={success} [{name}]",
                                start.map(|s| s.to_string()).unwrap_or_default()
                            );
                        }
                        _ => {}
                    }
                }
            }
        }

        // Sidebar view: summaries grouped into sections by is_phase_start
        println!("--- summary history (sidebar sections) ---");
        for s in cache.encounter_history.summaries() {
            if s.is_phase_start {
                println!("  ── {} ──", s.area_name);
            }
            println!(
                "     {} ({}s) {} {:?}",
                s.display_name,
                s.duration_seconds,
                s.start_time.as_deref().unwrap_or(""),
                s.pvp_outcome
            );
        }
    }
}
