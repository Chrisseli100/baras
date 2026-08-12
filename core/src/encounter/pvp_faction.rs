//! PvP friend/enemy inference
//!
//! PvP combat logs don't record teams, so factions are inferred transitively
//! from damage and healing between players, seeded by the local player:
//! damage always crosses factions, healing always stays within one. Once a
//! player is classified they stay classified for the rest of the match.

use hashbrown::HashMap;

pub use baras_types::PvpFaction;

use crate::combat_log::{CombatEvent, EntityType};
use crate::game_data::effect_id;

/// Transitive friend/enemy classifier for one PvP match.
#[derive(Debug, Clone, Default)]
pub struct PvpFactionTracker {
    factions: HashMap<i64, PvpFaction>,
}

impl PvpFactionTracker {
    /// Observe a combat event and classify unknown players where possible.
    /// Only player-to-player damage and healing carries faction information.
    pub fn observe(&mut self, event: &CombatEvent, local_player_id: i64) {
        let is_damage = event.effect.effect_id == effect_id::DAMAGE;
        if !is_damage && event.effect.effect_id != effect_id::HEAL {
            return;
        }

        let src = &event.source_entity;
        let tgt = &event.target_entity;
        if src.entity_type != EntityType::Player
            || tgt.entity_type != EntityType::Player
            || src.log_id == tgt.log_id
            || src.log_id == 0
            || tgt.log_id == 0
            || local_player_id == 0
        {
            return;
        }

        self.observe_pair(src.log_id, tgt.log_id, is_damage, local_player_id);
    }

    /// Core inference: damage ⇒ opposite factions, heal ⇒ same faction.
    /// When exactly one endpoint is known, the other is classified.
    /// Also used by the query layer to replay stored events.
    pub fn observe_pair(&mut self, src_id: i64, tgt_id: i64, is_damage: bool, local_player_id: i64) {
        let src = self.faction_of(src_id, local_player_id);
        let tgt = self.faction_of(tgt_id, local_player_id);

        match (src, tgt) {
            (Some(known), None) => {
                let f = if is_damage { known.opposite() } else { known };
                self.factions.insert(tgt_id, f);
            }
            (None, Some(known)) => {
                let f = if is_damage { known.opposite() } else { known };
                self.factions.insert(src_id, f);
            }
            // Both known (nothing to learn) or both unknown (no anchor yet)
            _ => {}
        }
    }

    /// Faction of a player, if inferred. The local player is always friendly.
    pub fn faction_of(&self, entity_id: i64, local_player_id: i64) -> Option<PvpFaction> {
        if entity_id == local_player_id {
            return Some(PvpFaction::Friendly);
        }
        self.factions.get(&entity_id).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCAL: i64 = 1;

    #[test]
    fn local_player_damage_and_heal_seed_factions() {
        let mut t = PvpFactionTracker::default();
        t.observe_pair(LOCAL, 2, true, LOCAL); // local damages 2 → enemy
        t.observe_pair(LOCAL, 3, false, LOCAL); // local heals 3 → friend
        assert_eq!(t.faction_of(2, LOCAL), Some(PvpFaction::Enemy));
        assert_eq!(t.faction_of(3, LOCAL), Some(PvpFaction::Friendly));
    }

    #[test]
    fn transitive_inference_through_known_players() {
        let mut t = PvpFactionTracker::default();
        t.observe_pair(LOCAL, 2, true, LOCAL); // 2 = enemy
        t.observe_pair(2, 4, false, LOCAL); // enemy heals 4 → enemy
        t.observe_pair(2, 5, true, LOCAL); // enemy damages 5 → friend
        t.observe_pair(5, 6, false, LOCAL); // friend heals 6 → friend
        t.observe_pair(6, 7, true, LOCAL); // friend damages 7 → enemy
        assert_eq!(t.faction_of(4, LOCAL), Some(PvpFaction::Enemy));
        assert_eq!(t.faction_of(5, LOCAL), Some(PvpFaction::Friendly));
        assert_eq!(t.faction_of(6, LOCAL), Some(PvpFaction::Friendly));
        assert_eq!(t.faction_of(7, LOCAL), Some(PvpFaction::Enemy));
    }

    #[test]
    fn reverse_inference_classifies_unknown_source() {
        let mut t = PvpFactionTracker::default();
        t.observe_pair(8, LOCAL, true, LOCAL); // 8 damages local → enemy
        t.observe_pair(9, LOCAL, false, LOCAL); // 9 heals local → friend
        assert_eq!(t.faction_of(8, LOCAL), Some(PvpFaction::Enemy));
        assert_eq!(t.faction_of(9, LOCAL), Some(PvpFaction::Friendly));
    }

    #[test]
    fn classification_is_sticky_and_unknown_pairs_learn_nothing() {
        let mut t = PvpFactionTracker::default();
        t.observe_pair(10, 11, true, LOCAL); // both unknown → nothing
        assert_eq!(t.faction_of(10, LOCAL), None);
        assert_eq!(t.faction_of(11, LOCAL), None);

        t.observe_pair(LOCAL, 10, false, LOCAL); // 10 = friend
        t.observe_pair(11, 10, true, LOCAL); // 11 damages friend → enemy
        // A later contradictory event must not reclassify
        t.observe_pair(LOCAL, 11, false, LOCAL); // would say friend — ignored, both known
        assert_eq!(t.faction_of(11, LOCAL), Some(PvpFaction::Enemy));
    }
}
