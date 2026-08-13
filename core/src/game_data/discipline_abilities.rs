//! Ability ID → discipline GUID map for ability-based discipline detection.
//! Used to infer disciplines for players that never emit DisciplineChanged
//! events (e.g., enemy players in PvP).
//!
//! Generated at build time from data/discipline_unique_abilities.csv.
include!(concat!(env!("OUT_DIR"), "/discipline_abilities.rs"));
