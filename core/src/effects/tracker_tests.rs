//! Tests for effect tracker
//!
//! Verifies instant alert behavior, OnApply alert fixes, and multi-healer
//! effect tracking correctness (exclusivity, refresh-after-removal, charges).

use chrono::Local;

use super::definition::EffectDefinition;
use super::tracker::{DefinitionSet, EffectTracker};
use crate::combat_log::EntityType;
use crate::context::empty_istr;
use crate::dsl::{AudioConfig, EffectSelector, EntityFilter, Trigger};
use crate::signal_processor::{GameSignal, SignalHandler};
use baras_types::{AlertTrigger, RefreshAbility, RefreshScope};

fn now() -> chrono::NaiveDateTime {
    Local::now().naive_local()
}

/// Create a minimal effect definition for testing
fn make_effect(
    id: &str,
    name: &str,
    trigger: Trigger,
    duration_secs: Option<f32>,
) -> EffectDefinition {
    EffectDefinition {
        id: id.to_string(),
        name: name.to_string(),
        display_text: None,
        enabled: true,
        trigger,
        ignore_effect_removed: false,
        refresh_abilities: vec![],
        is_aoe_refresh: false,
        aoe_refresh_immediate: false,
        is_refreshed_on_modify: false,
        default_charges: None,
        duration_secs,
        is_affected_by_alacrity: false,
        cooldown_ready_secs: 0.0,
        color: None,
        show_at_secs: 0.0,
        display_targets: vec![],
        icon_ability_id: None,
        show_icon: true,
        display_source: false,
        disciplines: vec![],
        source_disciplines: vec![],
        target_disciplines: vec![],
        ignore_refreshes: false,
        refresh_scope: RefreshScope::Both,
        persist_past_death: false,
        track_outside_combat: true,
        on_apply_trigger_timer: None,
        on_expire_trigger_timer: None,
        is_alert: false,
        alert_text: None,
        alert_on: AlertTrigger::None,
        alert_countdown_secs: None,
        audio: AudioConfig::default(),
        modifiers: vec![],
    }
}

fn make_tracker(defs: Vec<EffectDefinition>) -> EffectTracker {
    let mut def_set = DefinitionSet::new();
    def_set.add_definitions(defs, true);
    EffectTracker::new(def_set)
}

fn effect_applied_signal(effect_id: i64, timestamp: chrono::NaiveDateTime) -> GameSignal {
    GameSignal::EffectApplied {
        effect_id,
        effect_name: empty_istr(),
        action_id: 0,
        action_name: empty_istr(),
        source_id: 1,
        source_name: empty_istr(),
        source_entity_type: EntityType::Player,
        source_npc_id: 0,
        target_id: 2,
        target_name: empty_istr(),
        target_entity_type: EntityType::Player,
        target_npc_id: 0,
        timestamp,
        charges: None,
    }
}

fn ability_activated_signal(ability_id: i64, timestamp: chrono::NaiveDateTime) -> GameSignal {
    GameSignal::AbilityActivated {
        ability_id,
        ability_name: empty_istr(),
        source_id: 1,
        source_entity_type: EntityType::Player,
        source_name: empty_istr(),
        source_npc_id: 0,
        target_id: 2,
        target_name: empty_istr(),
        target_entity_type: EntityType::Player,
        target_npc_id: 0,
        timestamp,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Instant Alert: EffectApplied trigger
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_instant_alert_effect_applied_fires_alert_no_active_effect() {
    let mut def = make_effect(
        "test_alert",
        "Test Alert",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(12345)],
            source: EntityFilter::Any,
            target: EntityFilter::Any,
            position: vec![],
        },
        None,
    );
    def.is_alert = true;
    def.alert_text = Some("Danger!".to_string());

    let mut tracker = make_tracker(vec![def]);
    let ts = now();

    tracker.handle_signal(&effect_applied_signal(12345, ts), None);

    // Should fire alert
    let alerts = tracker.take_fired_alerts();
    assert_eq!(alerts.len(), 1, "Expected 1 fired alert");
    assert_eq!(alerts[0].text, "Danger!");
    assert!(alerts[0].alert_text_enabled);

    // Should NOT create an active effect
    let active_count = tracker.active_effects().count();
    assert_eq!(
        active_count, 0,
        "Instant alert should not create active effect"
    );
}

#[test]
fn test_instant_alert_no_text_when_alert_text_is_none() {
    let mut def = make_effect(
        "test_alert",
        "My Alert Name",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(12345)],
            source: EntityFilter::Any,
            target: EntityFilter::Any,
            position: vec![],
        },
        None,
    );
    def.is_alert = true;
    def.audio.enabled = true;
    def.audio.file = Some("beep.mp3".to_string());
    // alert_text is None — text overlay should NOT fire, but audio should

    let mut tracker = make_tracker(vec![def]);
    tracker.handle_signal(&effect_applied_signal(12345, now()), None);

    let alerts = tracker.take_fired_alerts();
    assert_eq!(alerts.len(), 1);
    // Text field is populated (for TTS fallback) but alert_text_enabled is false
    assert_eq!(alerts[0].text, "My Alert Name");
    assert!(
        !alerts[0].alert_text_enabled,
        "No text overlay when alert_text is None"
    );
    // Audio still fires
    assert!(alerts[0].audio_enabled);
    assert_eq!(alerts[0].audio_file.as_deref(), Some("beep.mp3"));
}

#[test]
fn test_instant_alert_carries_audio_config() {
    let mut def = make_effect(
        "test_alert",
        "Test Alert",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(12345)],
            source: EntityFilter::Any,
            target: EntityFilter::Any,
            position: vec![],
        },
        None,
    );
    def.is_alert = true;
    def.alert_text = Some("Watch out!".to_string());
    def.audio.enabled = true;
    def.audio.file = Some("warning.mp3".to_string());

    let mut tracker = make_tracker(vec![def]);
    tracker.handle_signal(&effect_applied_signal(12345, now()), None);

    let alerts = tracker.take_fired_alerts();
    assert_eq!(alerts.len(), 1);
    assert!(alerts[0].audio_enabled);
    assert_eq!(alerts[0].audio_file.as_deref(), Some("warning.mp3"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Instant Alert: AbilityCast trigger
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_instant_alert_ability_cast_fires_alert_no_active_effect() {
    let mut def = make_effect(
        "test_ability_alert",
        "Ability Alert",
        Trigger::AbilityCast {
            abilities: vec![crate::dsl::AbilitySelector::Id(99999)],
            source: EntityFilter::Any,
            target: EntityFilter::Any,
            position: vec![],
        },
        None,
    );
    def.is_alert = true;
    def.alert_text = Some("Ability fired!".to_string());

    let mut tracker = make_tracker(vec![def]);
    tracker.handle_signal(&ability_activated_signal(99999, now()), None);

    let alerts = tracker.take_fired_alerts();
    assert_eq!(alerts.len(), 1, "Expected 1 fired alert for ability cast");
    assert_eq!(alerts[0].text, "Ability fired!");

    let active_count = tracker.active_effects().count();
    assert_eq!(
        active_count, 0,
        "Instant alert should not create active effect"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Non-instant (is_alert=false) — regression tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_non_instant_effect_creates_active_effect() {
    let def = make_effect(
        "normal_effect",
        "Normal Effect",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(12345)],
            source: EntityFilter::Any,
            target: EntityFilter::Any,
            position: vec![],
        },
        Some(15.0),
    );

    let mut tracker = make_tracker(vec![def]);
    tracker.handle_signal(&effect_applied_signal(12345, now()), None);

    // Should create active effect
    let active_count = tracker.active_effects().count();
    assert_eq!(
        active_count, 1,
        "Normal effect should create an active effect"
    );

    // Should NOT fire an alert (alert_on is None)
    let alerts = tracker.take_fired_alerts();
    assert!(alerts.is_empty(), "No alert expected for alert_on=None");
}

// ─────────────────────────────────────────────────────────────────────────────
// OnApply alert fix for AbilityCast triggers
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_on_apply_alert_fires_for_ability_cast_trigger() {
    let mut def = make_effect(
        "proc_with_alert",
        "Proc Alert",
        Trigger::AbilityCast {
            abilities: vec![crate::dsl::AbilitySelector::Id(99999)],
            source: EntityFilter::Any,
            target: EntityFilter::Any,
            position: vec![],
        },
        Some(10.0),
    );
    def.alert_on = AlertTrigger::OnApply;
    def.alert_text = Some("Proc activated!".to_string());

    let mut tracker = make_tracker(vec![def]);
    tracker.handle_signal(&ability_activated_signal(99999, now()), None);

    // Should create active effect AND fire alert
    let active_count = tracker.active_effects().count();
    assert_eq!(active_count, 1, "Should create active effect");

    let alerts = tracker.take_fired_alerts();
    assert_eq!(
        alerts.len(),
        1,
        "Expected OnApply alert for ability cast trigger"
    );
    assert_eq!(alerts[0].text, "Proc activated!");
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: build signals with explicit source/target IDs
// ─────────────────────────────────────────────────────────────────────────────

fn effect_applied_signal_with_source(
    effect_id: i64,
    source_id: i64,
    target_id: i64,
    timestamp: chrono::NaiveDateTime,
) -> GameSignal {
    GameSignal::EffectApplied {
        effect_id,
        effect_name: empty_istr(),
        action_id: 0,
        action_name: empty_istr(),
        source_id,
        source_name: empty_istr(),
        source_entity_type: EntityType::Player,
        source_npc_id: 0,
        target_id,
        target_name: empty_istr(),
        target_entity_type: EntityType::Player,
        target_npc_id: 0,
        timestamp,
        charges: None,
    }
}

fn effect_removed_signal_with_source(
    effect_id: i64,
    source_id: i64,
    target_id: i64,
    timestamp: chrono::NaiveDateTime,
) -> GameSignal {
    GameSignal::EffectRemoved {
        effect_id,
        effect_name: empty_istr(),
        source_id,
        source_entity_type: EntityType::Player,
        source_name: empty_istr(),
        source_npc_id: 0,
        target_id,
        target_entity_type: EntityType::Player,
        target_name: empty_istr(),
        target_npc_id: 0,
        timestamp,
    }
}

fn ability_activated_signal_with_source(
    ability_id: i64,
    source_id: i64,
    target_id: i64,
    timestamp: chrono::NaiveDateTime,
) -> GameSignal {
    GameSignal::AbilityActivated {
        ability_id,
        ability_name: empty_istr(),
        source_id,
        source_entity_type: EntityType::Player,
        source_name: empty_istr(),
        source_npc_id: 0,
        target_id,
        target_name: empty_istr(),
        target_entity_type: EntityType::Player,
        target_npc_id: 0,
        timestamp,
    }
}

fn charges_changed_signal(
    effect_id: i64,
    source_id: i64,
    target_id: i64,
    charges: u8,
    timestamp: chrono::NaiveDateTime,
) -> GameSignal {
    GameSignal::EffectChargesChanged {
        effect_id,
        effect_name: empty_istr(),
        action_id: 0,
        action_name: empty_istr(),
        source_id,
        source_entity_type: EntityType::Player,
        source_name: empty_istr(),
        source_npc_id: 0,
        target_id,
        target_entity_type: EntityType::Player,
        target_name: empty_istr(),
        target_npc_id: 0,
        timestamp,
        charges,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Bug 1: Phantom _others effects from same-class refresh
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_kolto_shell_others_does_not_create_phantom_when_local_active() {
    // Set up kolto_shell (local_player) and kolto_shell_others (other_players)
    // both triggered by the same effect ID
    let effect_id: u64 = 985226842996736;
    let local_player_id: i64 = 1;
    let other_player_id: i64 = 99;
    let target_id: i64 = 2;

    let mut kolto_shell = make_effect(
        "kolto_shell",
        "Kolto Shell",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(effect_id)],
            source: EntityFilter::LocalPlayer,
            target: EntityFilter::AnyPlayer,
            position: vec![],
        },
        Some(180.0),
    );
    kolto_shell.display_targets = vec![super::definition::DisplayTarget::RaidFrames];
    kolto_shell.default_charges = Some(7);
    kolto_shell.refresh_abilities = vec![RefreshAbility::Simple(baras_types::AbilitySelector::Id(
        effect_id,
    ))];

    let kolto_shell_others = make_effect(
        "kolto_shell_others",
        "Other's Kolto Shell",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(effect_id)],
            source: EntityFilter::OtherPlayers,
            target: EntityFilter::AnyPlayer,
            position: vec![],
        },
        Some(180.0),
    );

    let mut tracker = make_tracker(vec![kolto_shell, kolto_shell_others]);
    tracker.set_player_context(local_player_id, 0);

    let ts = now();

    // Local player applies Kolto Shell on target
    tracker.handle_signal(
        &effect_applied_signal_with_source(effect_id as i64, local_player_id, target_id, ts),
        None,
    );

    assert_eq!(
        tracker.active_effects().count(),
        1,
        "Should have 1 effect after local player applies"
    );

    // Another player of same class casts Kolto Shell on same target (refresh in game)
    let ts2 = ts + chrono::Duration::seconds(5);
    tracker.handle_signal(
        &effect_applied_signal_with_source(effect_id as i64, other_player_id, target_id, ts2),
        None,
    );

    // Should still have only 1 effect (the local player's, refreshed)
    // NOT 2 effects (phantom kolto_shell_others should not be created)
    assert_eq!(
        tracker.active_effects().count(),
        1,
        "Should still have 1 effect — no phantom _others created"
    );

    // The existing effect should be the local player's (kolto_shell, not kolto_shell_others)
    let effect = tracker.active_effects().next().unwrap();
    assert_eq!(effect.definition_id, "kolto_shell");
    assert_eq!(effect.source_entity_id, local_player_id);
}

#[test]
fn test_kolto_shell_others_creates_normally_when_no_local_active() {
    // When local player's shell is NOT active, _others should create normally
    let effect_id: u64 = 985226842996736;
    let local_player_id: i64 = 1;
    let other_player_id: i64 = 99;
    let target_id: i64 = 2;

    let mut kolto_shell = make_effect(
        "kolto_shell",
        "Kolto Shell",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(effect_id)],
            source: EntityFilter::LocalPlayer,
            target: EntityFilter::AnyPlayer,
            position: vec![],
        },
        Some(180.0),
    );
    kolto_shell.display_targets = vec![super::definition::DisplayTarget::RaidFrames];

    let kolto_shell_others = make_effect(
        "kolto_shell_others",
        "Other's Kolto Shell",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(effect_id)],
            source: EntityFilter::OtherPlayers,
            target: EntityFilter::AnyPlayer,
            position: vec![],
        },
        Some(180.0),
    );

    let mut tracker = make_tracker(vec![kolto_shell, kolto_shell_others]);
    tracker.set_player_context(local_player_id, 0);

    // Other player applies without local player having it first
    let ts = now();
    tracker.handle_signal(
        &effect_applied_signal_with_source(effect_id as i64, other_player_id, target_id, ts),
        None,
    );

    assert_eq!(
        tracker.active_effects().count(),
        1,
        "Should create kolto_shell_others when no local shell active"
    );
    let effect = tracker.active_effects().next().unwrap();
    assert_eq!(effect.definition_id, "kolto_shell_others");
}

// ─────────────────────────────────────────────────────────────────────────────
// Bug 2: Refresh abilities resurrecting removed effects
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_refresh_does_not_resurrect_removed_effect() {
    // Simulate: Kolto Probes applied, then removed, then refresh ability cast.
    // The refresh should NOT bring back the removed effect.
    let effect_id: u64 = 814832605462528;
    let local_player_id: i64 = 1;
    let target_id: i64 = 2;

    let mut kolto_probe = make_effect(
        "kolto_probe",
        "Kolto Probe",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(effect_id)],
            source: EntityFilter::LocalPlayer,
            target: EntityFilter::AnyPlayer,
            position: vec![],
        },
        Some(21.0),
    );
    kolto_probe.display_targets = vec![super::definition::DisplayTarget::RaidFrames];
    kolto_probe.refresh_abilities = vec![RefreshAbility::Simple(baras_types::AbilitySelector::Id(
        effect_id,
    ))];

    let mut tracker = make_tracker(vec![kolto_probe]);
    tracker.set_player_context(local_player_id, 0);

    let ts = now();

    // Step 1: Apply effect
    tracker.handle_signal(
        &effect_applied_signal_with_source(effect_id as i64, local_player_id, target_id, ts),
        None,
    );
    assert_eq!(
        tracker.active_effects().count(),
        1,
        "Effect should be active"
    );

    // Step 2: Remove effect (authoritative game signal, well after application)
    let ts_remove = ts + chrono::Duration::seconds(22);
    tracker.handle_signal(
        &effect_removed_signal_with_source(effect_id as i64, local_player_id, target_id, ts_remove),
        None,
    );

    // The effect is marked removed but still in HashMap (tick() hasn't run yet to GC)
    // Verify it's marked removed
    let effect = tracker.active_effects().next().unwrap();
    assert!(
        effect.removed_at.is_some(),
        "Effect should be marked as removed"
    );

    // Step 3: Cast the refresh ability (slightly too late — probes already fell off)
    let ts_recast = ts_remove + chrono::Duration::milliseconds(500);
    tracker.handle_signal(
        &ability_activated_signal_with_source(
            effect_id as i64,
            local_player_id,
            target_id,
            ts_recast,
        ),
        None,
    );

    // The removed effect should NOT be resurrected by the refresh
    let effect = tracker.active_effects().next().unwrap();
    assert!(
        effect.removed_at.is_some(),
        "Removed effect should NOT be resurrected by refresh ability"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Bug 3: Charges from another player updating local effect
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_charges_from_other_player_do_not_update_local_effect() {
    // Two operative healers: both have Kolto Probes on the same target.
    // Charges changed from the other player should NOT update the local player's effect.
    let effect_id: u64 = 814832605462528;
    let local_player_id: i64 = 1;
    let other_player_id: i64 = 99;
    let target_id: i64 = 2;

    let kolto_probe = make_effect(
        "kolto_probe",
        "Kolto Probe",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(effect_id)],
            source: EntityFilter::LocalPlayer,
            target: EntityFilter::AnyPlayer,
            position: vec![],
        },
        Some(21.0),
    );

    let mut tracker = make_tracker(vec![kolto_probe]);
    tracker.set_player_context(local_player_id, 0);

    let ts = now();

    // Local player applies 2 stacks
    tracker.handle_signal(
        &effect_applied_signal_with_source(effect_id as i64, local_player_id, target_id, ts),
        None,
    );
    let ts2 = ts + chrono::Duration::seconds(1);
    tracker.handle_signal(
        &charges_changed_signal(effect_id as i64, local_player_id, target_id, 2, ts2),
        None,
    );

    let effect = tracker.active_effects().next().unwrap();
    assert_eq!(effect.stacks, 2, "Should have 2 stacks from local player");

    // Other player's probes cause a ModifyCharges event (from their source)
    let ts3 = ts + chrono::Duration::seconds(3);
    tracker.handle_signal(
        &charges_changed_signal(effect_id as i64, other_player_id, target_id, 4, ts3),
        None,
    );

    // Local player's effect should still show 2 stacks, NOT 4
    let effect = tracker.active_effects().next().unwrap();
    assert_eq!(
        effect.stacks, 2,
        "Charges from other player should NOT update local effect"
    );
}

#[test]
fn test_charges_from_local_player_do_update_local_effect() {
    // Sanity check: charges from the local player should still update
    let effect_id: u64 = 814832605462528;
    let local_player_id: i64 = 1;
    let target_id: i64 = 2;

    let kolto_probe = make_effect(
        "kolto_probe",
        "Kolto Probe",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(effect_id)],
            source: EntityFilter::LocalPlayer,
            target: EntityFilter::AnyPlayer,
            position: vec![],
        },
        Some(21.0),
    );

    let mut tracker = make_tracker(vec![kolto_probe]);
    tracker.set_player_context(local_player_id, 0);

    let ts = now();

    // Apply effect
    tracker.handle_signal(
        &effect_applied_signal_with_source(effect_id as i64, local_player_id, target_id, ts),
        None,
    );

    // Charges changed from local player
    let ts2 = ts + chrono::Duration::seconds(1);
    tracker.handle_signal(
        &charges_changed_signal(effect_id as i64, local_player_id, target_id, 2, ts2),
        None,
    );

    let effect = tracker.active_effects().next().unwrap();
    assert_eq!(
        effect.stacks, 2,
        "Charges from local player should update normally"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Source-agnostic refresh: other players' effects refresh via AbilityActivated
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_kolto_shell_others_refreshes_via_ability_activated() {
    // When another player recasts Kolto Shell, the existing kolto_shell_others
    // effect should get its timer refreshed via the AbilityActivated signal.
    let effect_id: u64 = 985226842996736;
    let local_player_id: i64 = 1;
    let other_player_id: i64 = 99;
    let target_id: i64 = 2;

    let kolto_shell = make_effect(
        "kolto_shell",
        "Kolto Shell",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(effect_id)],
            source: EntityFilter::LocalPlayer,
            target: EntityFilter::AnyPlayer,
            position: vec![],
        },
        Some(180.0),
    );

    let mut kolto_shell_others = make_effect(
        "kolto_shell_others",
        "Other's Kolto Shell",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(effect_id)],
            source: EntityFilter::OtherPlayers,
            target: EntityFilter::AnyPlayer,
            position: vec![],
        },
        Some(180.0),
    );
    kolto_shell_others.display_targets = vec![super::definition::DisplayTarget::RaidFrames];
    kolto_shell_others.refresh_abilities = vec![RefreshAbility::Simple(
        baras_types::AbilitySelector::Id(effect_id),
    )];

    let mut tracker = make_tracker(vec![kolto_shell, kolto_shell_others]);
    tracker.set_player_context(local_player_id, 0);

    let ts = now();

    // Other player applies Kolto Shell on target (creates kolto_shell_others)
    tracker.handle_signal(
        &effect_applied_signal_with_source(effect_id as i64, other_player_id, target_id, ts),
        None,
    );

    assert_eq!(tracker.active_effects().count(), 1);
    let effect = tracker.active_effects().next().unwrap();
    assert_eq!(effect.definition_id, "kolto_shell_others");
    let original_expires = effect.expires_at;

    // 60 seconds later, other player recasts (AbilityActivated signal)
    let ts2 = ts + chrono::Duration::seconds(60);
    tracker.handle_signal(
        &ability_activated_signal_with_source(effect_id as i64, other_player_id, target_id, ts2),
        None,
    );

    // The effect should still exist and its timer should be refreshed
    assert_eq!(tracker.active_effects().count(), 1);
    let effect = tracker.active_effects().next().unwrap();
    assert_eq!(effect.definition_id, "kolto_shell_others");
    assert!(
        effect.expires_at > original_expires,
        "Timer should be refreshed — expires_at should be later than original"
    );
    assert_eq!(
        effect.last_refreshed_at, ts2,
        "last_refreshed_at should match the recast timestamp"
    );
}

#[test]
fn test_other_player_effect_late_registration_not_marked_local() {
    // When an effect is late-registered via refresh for another player,
    // it should NOT be marked as is_from_local_player.
    let effect_id: u64 = 985226842996736;
    let local_player_id: i64 = 1;
    let other_player_id: i64 = 99;
    let target_id: i64 = 2;

    let mut kolto_shell_others = make_effect(
        "kolto_shell_others",
        "Other's Kolto Shell",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(effect_id)],
            source: EntityFilter::OtherPlayers,
            target: EntityFilter::AnyPlayer,
            position: vec![],
        },
        Some(180.0),
    );
    kolto_shell_others.display_targets = vec![super::definition::DisplayTarget::RaidFrames];
    kolto_shell_others.refresh_abilities = vec![RefreshAbility::Simple(
        baras_types::AbilitySelector::Id(effect_id),
    )];

    let mut tracker = make_tracker(vec![kolto_shell_others]);
    tracker.set_player_context(local_player_id, 0);

    // No EffectApplied — go straight to AbilityActivated (late registration)
    let ts = now();
    tracker.handle_signal(
        &ability_activated_signal_with_source(effect_id as i64, other_player_id, target_id, ts),
        None,
    );

    assert_eq!(tracker.active_effects().count(), 1);
    let effect = tracker.active_effects().next().unwrap();
    assert_eq!(effect.definition_id, "kolto_shell_others");
    assert!(
        !effect.is_from_local_player,
        "Late-registered effect from other player should NOT be marked as local"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Modifier refill_duration must work without is_refreshed_on_modify
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_charges_modifier_refills_duration_without_refresh_on_modify() {
    // Tactical Advantage style: duration refills only when charges INCREASE,
    // so is_refreshed_on_modify stays off and a SelfChargesChanged(Increased)
    // modifier with refill_duration handles the refresh instead.
    let effect_id: u64 = 12345;
    let local_player_id: i64 = 1;

    let mut def = make_effect(
        "tactical_advantage",
        "Tactical Advantage",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(effect_id)],
            source: EntityFilter::LocalPlayer,
            target: EntityFilter::LocalPlayer,
            position: vec![],
        },
        Some(10.0),
    );
    def.modifiers = vec![baras_types::EffectModifier {
        trigger: baras_types::Trigger::SelfChargesChanged {
            direction: Some(baras_types::ChargeDirection::Increased),
        },
        adjust_duration_secs: 0.0,
        requires_crit: false,
        refill_duration: true,
        icd_secs: None,
        icd_from_application: false,
        icd_affected_by_alacrity: false,
        max_duration_secs: None,
        cancel: false,
    }];

    let mut tracker = make_tracker(vec![def]);
    tracker.set_player_context(local_player_id, 0);

    let ts = now();
    tracker.handle_signal(
        &effect_applied_signal_with_source(effect_id as i64, local_player_id, local_player_id, ts),
        None,
    );
    let original_expires = tracker.active_effects().next().unwrap().expires_at;

    // Charges increase 4s later — duration should refill from that timestamp
    let ts2 = ts + chrono::Duration::seconds(4);
    tracker.handle_signal(
        &charges_changed_signal(effect_id as i64, local_player_id, local_player_id, 2, ts2),
        None,
    );
    let effect = tracker.active_effects().next().unwrap();
    assert_eq!(effect.stacks, 2);
    assert!(
        effect.expires_at > original_expires,
        "refill_duration modifier should extend expires_at on charge increase"
    );
    let refilled_expires = effect.expires_at;

    // Charges decrease — Increased direction filter must NOT refill
    let ts3 = ts + chrono::Duration::seconds(6);
    tracker.handle_signal(
        &charges_changed_signal(effect_id as i64, local_player_id, local_player_id, 1, ts3),
        None,
    );
    let effect = tracker.active_effects().next().unwrap();
    assert_eq!(effect.stacks, 1);
    assert_eq!(
        effect.expires_at, refilled_expires,
        "charge decrease should not refill duration"
    );
}

#[test]
fn test_local_player_cast_does_not_create_phantom_others_via_refresh() {
    // When the LOCAL player casts Kolto Shell, the AbilityActivated signal should
    // NOT create a phantom kolto_shell_others via late registration in the refresh path.
    // The source filter (source = "other_players") must reject the local player.
    let effect_id: u64 = 985226842996736;
    let local_player_id: i64 = 1;
    let target_id: i64 = 2;

    let mut kolto_shell = make_effect(
        "kolto_shell",
        "Kolto Shell",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(effect_id)],
            source: EntityFilter::LocalPlayer,
            target: EntityFilter::AnyPlayer,
            position: vec![],
        },
        Some(180.0),
    );
    kolto_shell.display_targets = vec![super::definition::DisplayTarget::RaidFrames];
    kolto_shell.refresh_abilities = vec![RefreshAbility::Simple(baras_types::AbilitySelector::Id(
        effect_id,
    ))];

    let mut kolto_shell_others = make_effect(
        "kolto_shell_others",
        "Other's Kolto Shell",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(effect_id)],
            source: EntityFilter::OtherPlayers,
            target: EntityFilter::AnyPlayer,
            position: vec![],
        },
        Some(180.0),
    );
    kolto_shell_others.display_targets = vec![super::definition::DisplayTarget::RaidFrames];
    kolto_shell_others.refresh_abilities = vec![RefreshAbility::Simple(
        baras_types::AbilitySelector::Id(effect_id),
    )];

    let mut tracker = make_tracker(vec![kolto_shell, kolto_shell_others]);
    tracker.set_player_context(local_player_id, 0);

    let ts = now();

    // Local player applies Kolto Shell (EffectApplied creates kolto_shell)
    tracker.handle_signal(
        &effect_applied_signal_with_source(effect_id as i64, local_player_id, target_id, ts),
        None,
    );

    assert_eq!(
        tracker.active_effects().count(),
        1,
        "Should have exactly 1 effect (kolto_shell)"
    );
    assert_eq!(
        tracker.active_effects().next().unwrap().definition_id,
        "kolto_shell"
    );

    // Local player recasts (AbilityActivated) — should refresh kolto_shell,
    // NOT create a phantom kolto_shell_others
    let ts2 = ts + chrono::Duration::seconds(30);
    tracker.handle_signal(
        &ability_activated_signal_with_source(effect_id as i64, local_player_id, target_id, ts2),
        None,
    );

    assert_eq!(
        tracker.active_effects().count(),
        1,
        "Should still have exactly 1 effect — no phantom kolto_shell_others"
    );
    let effect = tracker.active_effects().next().unwrap();
    assert_eq!(effect.definition_id, "kolto_shell");
    assert_eq!(
        effect.last_refreshed_at, ts2,
        "kolto_shell should be refreshed"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// ResourceSpent modifier + cancel
// ─────────────────────────────────────────────────────────────────────────

fn resource_spent_signal(source_id: i64, amount: f32, timestamp: chrono::NaiveDateTime) -> GameSignal {
    GameSignal::ResourceSpent {
        source_id,
        source_entity_type: EntityType::Player,
        amount,
        timestamp,
    }
}

fn spend_modifier_def(modifier: baras_types::EffectModifier) -> (u64, i64, EffectTracker) {
    let effect_id: u64 = 4242;
    let local_player_id: i64 = 1;
    let mut def = make_effect(
        "spend_defensive",
        "Spend Defensive",
        Trigger::EffectApplied {
            effects: vec![EffectSelector::Id(effect_id)],
            source: EntityFilter::LocalPlayer,
            target: EntityFilter::LocalPlayer,
            position: vec![],
        },
        Some(10.0),
    );
    def.modifiers = vec![modifier];
    let mut tracker = make_tracker(vec![def]);
    tracker.set_player_context(local_player_id, 0);
    (effect_id, local_player_id, tracker)
}

#[test]
fn test_resource_spent_modifier_scales_by_amount() {
    let (effect_id, player, mut tracker) = spend_modifier_def(baras_types::EffectModifier {
        trigger: baras_types::Trigger::ResourceSpent { per_amount: 10.0 },
        adjust_duration_secs: 1.0,
        requires_crit: false,
        refill_duration: false,
        icd_secs: None,
        icd_from_application: false,
        icd_affected_by_alacrity: false,
        max_duration_secs: None,
        cancel: false,
    });

    let ts = now();
    tracker.handle_signals(
        &[effect_applied_signal_with_source(effect_id as i64, player, player, ts)],
        None,
    );
    let original = tracker.active_effects().next().unwrap().expires_at.unwrap();

    // 44 spent at 1s/10 → +4.4s
    let ts2 = ts + chrono::Duration::seconds(1);
    tracker.handle_signals(&[resource_spent_signal(player, 44.0, ts2)], None);
    let extended = tracker.active_effects().next().unwrap().expires_at.unwrap();
    assert_eq!((extended - original).num_milliseconds(), 4400);

    // Spend by a different entity must not proc
    tracker.handle_signals(&[resource_spent_signal(99, 100.0, ts2)], None);
    assert_eq!(tracker.active_effects().next().unwrap().expires_at.unwrap(), extended);
}

#[test]
fn test_resource_spent_modifier_cancels_effect() {
    let (effect_id, player, mut tracker) = spend_modifier_def(baras_types::EffectModifier {
        trigger: baras_types::Trigger::ResourceSpent { per_amount: 0.0 },
        adjust_duration_secs: 0.0,
        requires_crit: false,
        refill_duration: false,
        icd_secs: None,
        icd_from_application: false,
        icd_affected_by_alacrity: false,
        max_duration_secs: None,
        cancel: true,
    });

    let ts = now();
    tracker.handle_signals(
        &[effect_applied_signal_with_source(effect_id as i64, player, player, ts)],
        None,
    );
    assert!(tracker.active_effects().next().unwrap().removed_at.is_none());

    tracker.handle_signals(&[resource_spent_signal(player, 5.0, ts + chrono::Duration::seconds(1))], None);
    assert!(
        tracker.active_effects().next().unwrap().removed_at.is_some(),
        "cancel modifier should mark the effect removed"
    );
}

#[test]
fn test_killing_blow_modifier_extends_effect() {
    let (effect_id, player, mut tracker) = spend_modifier_def(baras_types::EffectModifier {
        trigger: baras_types::Trigger::KillingBlow,
        adjust_duration_secs: 3.0,
        requires_crit: false,
        refill_duration: false,
        icd_secs: None,
        icd_from_application: false,
        icd_affected_by_alacrity: false,
        max_duration_secs: None,
        cancel: false,
    });
    let ts = now();
    tracker.handle_signals(
        &[effect_applied_signal_with_source(effect_id as i64, player, player, ts)],
        None,
    );
    let original = tracker.active_effects().next().unwrap().expires_at.unwrap();

    let death = |killer_id: i64, t| GameSignal::EntityDeath {
        entity_id: 500,
        entity_type: EntityType::Npc,
        npc_id: 123,
        entity_name: "Add".to_string(),
        killer_id,
        timestamp: t,
    };
    // Kill by someone else: no proc
    tracker.handle_signals(&[death(99, ts + chrono::Duration::seconds(1))], None);
    assert_eq!(tracker.active_effects().next().unwrap().expires_at.unwrap(), original);
    // Kill by the holder: +3s
    tracker.handle_signals(&[death(player, ts + chrono::Duration::seconds(2))], None);
    let extended = tracker.active_effects().next().unwrap().expires_at.unwrap();
    assert_eq!((extended - original).num_milliseconds(), 3000);
}

#[test]
fn test_any_of_modifier_shares_icd_across_trigger_kinds() {
    let (effect_id, player, mut tracker) = spend_modifier_def(baras_types::EffectModifier {
        trigger: baras_types::Trigger::AnyOf {
            conditions: vec![
                baras_types::Trigger::ResourceSpent { per_amount: 0.0 },
                baras_types::Trigger::KillingBlow,
            ],
        },
        adjust_duration_secs: 2.0,
        requires_crit: false,
        refill_duration: false,
        icd_secs: Some(5.0),
        icd_from_application: false,
        icd_affected_by_alacrity: false,
        max_duration_secs: None,
        cancel: false,
    });
    let ts = now();
    tracker.handle_signals(
        &[effect_applied_signal_with_source(effect_id as i64, player, player, ts)],
        None,
    );
    let original = tracker.active_effects().next().unwrap().expires_at.unwrap();

    // Resource spend procs (+2s)
    tracker.handle_signals(&[resource_spent_signal(player, 5.0, ts + chrono::Duration::seconds(1))], None);
    let after_spend = tracker.active_effects().next().unwrap().expires_at.unwrap();
    assert_eq!((after_spend - original).num_milliseconds(), 2000);

    // Killing blow 1s later is blocked by the ICD shared with the spend
    let death = GameSignal::EntityDeath {
        entity_id: 500,
        entity_type: EntityType::Npc,
        npc_id: 123,
        entity_name: "Add".to_string(),
        killer_id: player,
        timestamp: ts + chrono::Duration::seconds(2),
    };
    tracker.handle_signals(&[death], None);
    assert_eq!(tracker.active_effects().next().unwrap().expires_at.unwrap(), after_spend);

    // After the ICD elapses, the killing blow procs
    let death = GameSignal::EntityDeath {
        entity_id: 501,
        entity_type: EntityType::Npc,
        npc_id: 123,
        entity_name: "Add".to_string(),
        killer_id: player,
        timestamp: ts + chrono::Duration::seconds(7),
    };
    tracker.handle_signals(&[death], None);
    let after_kill = tracker.active_effects().next().unwrap().expires_at.unwrap();
    assert_eq!((after_kill - after_spend).num_milliseconds(), 2000);
}

#[test]
fn test_icd_from_application_gates_first_proc() {
    let (effect_id, player, mut tracker) = spend_modifier_def(baras_types::EffectModifier {
        trigger: baras_types::Trigger::ResourceSpent { per_amount: 0.0 },
        adjust_duration_secs: 2.0,
        requires_crit: false,
        refill_duration: false,
        icd_secs: Some(3.0),
        icd_from_application: true,
        icd_affected_by_alacrity: false,
        max_duration_secs: None,
        cancel: false,
    });
    let ts = now();
    tracker.handle_signals(
        &[effect_applied_signal_with_source(effect_id as i64, player, player, ts)],
        None,
    );
    let original = tracker.active_effects().next().unwrap().expires_at.unwrap();

    // 1s after application: still inside the ICD window counted from applied_at
    tracker.handle_signals(&[resource_spent_signal(player, 5.0, ts + chrono::Duration::seconds(1))], None);
    assert_eq!(tracker.active_effects().next().unwrap().expires_at.unwrap(), original);

    // 3s after application: allowed
    tracker.handle_signals(&[resource_spent_signal(player, 5.0, ts + chrono::Duration::seconds(3))], None);
    let extended = tracker.active_effects().next().unwrap().expires_at.unwrap();
    assert_eq!((extended - original).num_milliseconds(), 2000);
}

#[test]
fn test_modifier_revives_interpolation_expired_effect() {
    let (effect_id, player, mut tracker) = spend_modifier_def(baras_types::EffectModifier {
        trigger: baras_types::Trigger::ResourceSpent { per_amount: 0.0 },
        adjust_duration_secs: 5.0,
        requires_crit: false,
        refill_duration: false,
        icd_secs: None,
        icd_from_application: false,
        icd_affected_by_alacrity: false,
        max_duration_secs: None,
        cancel: false,
    });
    let ts = now();
    tracker.handle_signals(
        &[effect_applied_signal_with_source(effect_id as i64, player, player, ts)],
        None,
    );
    let original = tracker.active_effects().next().unwrap().expires_at.unwrap();

    // Simulate tick() having expired the effect on interpolated time while
    // the log (e.g. a delayed flush) hadn't caught up yet.
    tracker.active_effects_mut().next().unwrap().timer_expired = true;

    // A log event from BEFORE expires_at proves the effect was still alive.
    tracker.handle_signals(&[resource_spent_signal(player, 5.0, ts + chrono::Duration::seconds(8))], None);
    let e = tracker.active_effects().next().unwrap();
    assert!(!e.timer_expired, "modifier should revive an interpolation-expired effect");
    assert_eq!((e.expires_at.unwrap() - original).num_milliseconds(), 5000);

    // A log event from AFTER expires_at must not resurrect a genuinely expired effect.
    tracker.active_effects_mut().next().unwrap().timer_expired = true;
    let late = tracker.active_effects().next().unwrap().expires_at.unwrap() + chrono::Duration::seconds(1);
    tracker.handle_signals(&[resource_spent_signal(player, 5.0, late)], None);
    assert!(tracker.active_effects().next().unwrap().timer_expired);
}

#[test]
fn test_icd_affected_by_alacrity_shortens_cooldown() {
    // 25% alacrity: 10s ICD → 8s
    let (effect_id, player, mut tracker) = spend_modifier_def(baras_types::EffectModifier {
        trigger: baras_types::Trigger::ResourceSpent { per_amount: 0.0 },
        adjust_duration_secs: 1.0,
        requires_crit: false,
        refill_duration: false,
        icd_secs: Some(10.0),
        icd_from_application: true,
        icd_affected_by_alacrity: true,
        max_duration_secs: Some(60.0),
        cancel: false,
    });
    tracker.set_alacrity(25.0);
    let ts = now();
    tracker.handle_signals(
        &[effect_applied_signal_with_source(effect_id as i64, player, player, ts)],
        None,
    );
    let original = tracker.active_effects().next().unwrap().expires_at.unwrap();

    // 7s: inside the 8s scaled ICD
    tracker.handle_signals(&[resource_spent_signal(player, 1.0, ts + chrono::Duration::seconds(7))], None);
    assert_eq!(tracker.active_effects().next().unwrap().expires_at.unwrap(), original);

    // 8s: scaled ICD elapsed (would still be blocked by the raw 10s)
    tracker.handle_signals(&[resource_spent_signal(player, 1.0, ts + chrono::Duration::seconds(8))], None);
    let extended = tracker.active_effects().next().unwrap().expires_at.unwrap();
    assert_eq!((extended - original).num_milliseconds(), 1000);
}
