//! PvP area identification
//!
//! Area IDs for PvP instances (warzones, arenas)

/// PvP instance kind: 8-player warzones vs 4-player arenas
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PvpAreaKind {
    Warzone,
    Arena,
}

/// Classify a PvP area ID, or `None` if the area is not a PvP instance
pub fn pvp_area_kind(area_id: i64) -> Option<PvpAreaKind> {
    match area_id {
        // Warzones (8-player)
        137438956902      // The Civil War (Alderaan)
        | 945872057669561 // Yavin Ruins
        | 137438992654    // Novare Coast
        | 137438993104    // Ancient Hypergate
        | 137438989517    // The Voidstar
        | 833571547775744 // Odessen Proving Grounds
        | 137438988866    // The Pit (Nar Shaddaa Huttball)
        | 137438953518    // Quesh Huttball Pit
        | 833571547775746 // Vandin Huttball (The Sky Shredder)
        => Some(PvpAreaKind::Warzone),

        // Arenas (4-player)
        137438993370      // Corellia Square
        | 137438993381    // Makeb Mesa Arena
        | 945872057669563 // Mandalorian Battle Ring
        | 137438993376    // Orbital Station
        | 833571547775745 // Rishi Cove
        | 137438993374    // Tatooine Canyon
        | 945872057669629 // Onderon Palatial Ruins
        => Some(PvpAreaKind::Arena),

        _ => None,
    }
}

/// Check if an area ID corresponds to a PvP instance
pub fn is_pvp_area(area_id: i64) -> bool {
    pvp_area_kind(area_id).is_some()
}

/// System abilities whose Heal events mark an arena round boundary: the game
/// heals every player to full the instant a round ends ("Heal Target"), and
/// additionally fires "Rebirth" when the final round ends the match (a few ms
/// before the Heal Target burst). Neither occurs mid-round.
pub const ARENA_ROUND_END_ABILITY_IDS: [i64; 2] = [
    773652459028480, // Rebirth (match end only)
    813707324030976, // Heal Target (every round end)
];

/// Display label for a PvP area's match type
pub fn pvp_match_label(area_id: i64) -> &'static str {
    match pvp_area_kind(area_id) {
        Some(PvpAreaKind::Warzone) => "Warzone Match",
        Some(PvpAreaKind::Arena) => "Arena Round",
        None => "PvP Match",
    }
}
