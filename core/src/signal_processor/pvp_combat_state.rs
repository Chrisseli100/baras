//! PvP combat state handling.
//!
//! PvP instances ignore all PvE encounter boundary logic — inactivity timeouts,
//! revive-immunity splits, wipe detection, and ExitCombat ends don't apply.
//! `combat_state::advance_combat_state` dispatches here for InCombat encounters
//! whose area is a PvP instance.

use crate::combat_log::CombatEvent;
use crate::encounter::EncounterState;
use crate::game_data::{effect_id, effect_type_id};
use crate::state::SessionCache;

use super::GameSignal;

/// Advance an InCombat warzone encounter.
///
/// A warzone match is one contiguous encounter: EnterCombat/ExitCombat from
/// respawns and rejoins never split it. The only boundary is zoning out
/// (AreaEntered with a different area ID); the encounter end is then backdated
/// to the final ExitCombat seen, so scoreboard idle time isn't counted.
pub fn handle_in_combat_warzone(
    event: &CombatEvent,
    cache: &mut SessionCache,
) -> (Vec<GameSignal>, bool) {
    let mut signals = Vec::new();
    let mut was_accumulated = false;
    let timestamp = event.timestamp;

    // Zone-out ends the match. Re-entering the SAME area (e.g. a Voidstar
    // round swap) is not a zone-out — the match continues.
    let zoned_out = event.effect.type_id == effect_type_id::AREAENTERED
        && cache
            .current_encounter()
            .and_then(|e| e.area_id)
            .is_some_and(|id| id != event.effect.effect_id);

    if zoned_out {
        let encounter_id = cache.current_encounter().map(|e| e.id).unwrap_or(0);
        let mut exit_time = timestamp;

        if let Some(enc) = cache.current_encounter_mut() {
            exit_time = enc.last_exit_combat_time.unwrap_or(timestamp);
            enc.exit_combat_time = Some(exit_time);
            enc.state = EncounterState::PostCombat { exit_time };
            let duration = enc.duration_seconds(None).unwrap_or(0) as f32;
            enc.challenge_tracker.finalize(exit_time, duration);
        }

        tracing::info!(
            "[ENCOUNTER] Zoned out of warzone at {}, ending encounter {} at {}",
            timestamp,
            encounter_id,
            exit_time
        );

        signals.push(GameSignal::CombatEnded {
            timestamp: exit_time,
            encounter_id,
            // Win/loss isn't derivable from the combat log; a match played to
            // its end (zone-out) is not a wipe.
            success: true,
        });

        cache.push_new_encounter();
        return (signals, was_accumulated);
    }

    // Everything else is part of the match.
    if let Some(enc) = cache.current_encounter_mut() {
        enc.track_event_entities(event);
        enc.accumulate_data(event);
        enc.track_event_line(event.line_number);
        was_accumulated = true;

        if event.effect.effect_id == effect_id::DAMAGE {
            enc.last_damage_time = Some(timestamp);
        } else if event.effect.effect_id == effect_id::EXITCOMBAT {
            // Doesn't end the match, but the final one becomes the encounter
            // end when the player zones out.
            enc.last_exit_combat_time = Some(timestamp);
        }
    }

    (signals, was_accumulated)
}
