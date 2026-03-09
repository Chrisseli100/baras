//! Entity editing tab
//!
//! CRUD for boss entity (NPC) roster definitions.
//! Entities define which NPCs are bosses, adds, triggers, and kill targets.
//! Uses EntityDefinition DSL type directly.

use dioxus::prelude::*;

use crate::api;
use crate::types::{BossWithPath, EncounterItem, EntityDefinition};

use super::{IdChipEditor, InlineNameCreator, NpcIdChipEditor};

// ─────────────────────────────────────────────────────────────────────────────
// Entities Tab
// ─────────────────────────────────────────────────────────────────────────────

/// Create a default entity definition
fn default_entity(name: String) -> EntityDefinition {
    EntityDefinition {
        name,
        ids: vec![],
        is_boss: false,
        triggers_encounter: None, // Uses is_boss default
        is_kill_target: false,
        show_on_hp_overlay: None, // Uses is_boss default
        hp_markers: vec![],
        shields: vec![],
    }
}

#[component]
pub fn EntitiesTab(
    boss_with_path: BossWithPath,
    expanded_entity: Signal<Option<String>>,
    on_change: EventHandler<Vec<EntityDefinition>>,
    on_status: EventHandler<(String, bool)>,
) -> Element {
    // Extract entities from BossWithPath
    let entities = boss_with_path.boss.entities.clone();

    rsx! {
        div { class: "entities-tab",
            // Header
            div { class: "flex items-center justify-between mb-sm",
                span { class: "text-sm text-secondary", "{entities.len()} entities" }
                {
                    let bwp = boss_with_path.clone();
                    let entities_for_create = entities.clone();
                    rsx! {
                        InlineNameCreator {
                            button_label: "+ New Entity",
                            placeholder: "Entity name...",
                            on_create: move |name: String| {
                                let entities_clone = entities_for_create.clone();
                                let boss_id = bwp.boss.id.clone();
                                let file_path = bwp.file_path.clone();
                                let entity = default_entity(name);
                                let item = EncounterItem::Entity(entity);
                                spawn(async move {
                                    match api::create_encounter_item(&boss_id, &file_path, &item).await {
                                        Ok(EncounterItem::Entity(created)) => {
                                            let created_name = created.name.clone();
                                            let mut current = entities_clone;
                                            current.push(created);
                                            on_change.call(current);
                                            expanded_entity.set(Some(created_name));
                                            on_status.call(("Created".to_string(), false));
                                        }
                                        Ok(_) => on_status.call(("Unexpected response type".to_string(), true)),
                                        Err(e) => on_status.call((e, true)),
                                    }
                                });
                            }
                        }
                    }
                }
            }

            // Help text
            div { class: "text-xs text-muted mb-sm",
                "Add NPCs to the entity roster by game ids. Entity roster names can be used as selectors for source/target filter conditions."
            }

            // Entity list
            if entities.is_empty() {
                div { class: "empty-state text-sm", "No entities defined" }
            } else {
                for entity in entities.clone() {
                    {
                        let entity_key = entity.name.clone();
                        let is_expanded = expanded_entity() == Some(entity_key.clone());
                        let entities_for_row = entities.clone();

                        rsx! {
                            EntityRow {
                                key: "{entity_key}",
                                entity: entity.clone(),
                                boss_with_path: boss_with_path.clone(),
                                expanded: is_expanded,
                                on_toggle: move |_| {
                                    expanded_entity.set(if is_expanded { None } else { Some(entity_key.clone()) });
                                },
                                on_change: on_change,
                                on_status: on_status,
                                on_collapse: move |_| expanded_entity.set(None),
                                all_entities: entities_for_row,
                            }
                        }
                    }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Entity Row
// ─────────────────────────────────────────────────────────────────────────────

#[component]
fn EntityRow(
    entity: EntityDefinition,
    boss_with_path: BossWithPath,
    expanded: bool,
    all_entities: Vec<EntityDefinition>,
    on_toggle: EventHandler<()>,
    on_change: EventHandler<Vec<EntityDefinition>>,
    on_status: EventHandler<(String, bool)>,
    on_collapse: EventHandler<()>,
) -> Element {
    let mut is_dirty = use_signal(|| false);
    let id_count = entity.ids.len();

    // Extract context for API calls
    let boss_id = boss_with_path.boss.id.clone();
    let file_path = boss_with_path.file_path.clone();

    rsx! {
        div { class: "list-item",
            // Header row
            div {
                class: "list-item-header",
                onclick: move |_| on_toggle.call(()),
                span { class: "list-item-expand", if expanded { "▼" } else { "▶" } }
                span { class: "font-medium", "{entity.name}" }
                if expanded && is_dirty() {
                    span { class: "unsaved-indicator", title: "Unsaved changes" }
                }
                span { class: "text-xs text-muted text-mono", "{id_count} IDs" }
                if entity.is_boss {
                    span { class: "tag tag-danger", "Boss" }
                }
                // triggers_encounter defaults to is_boss when None
                if entity.triggers_encounter.unwrap_or(entity.is_boss) {
                    span { class: "tag tag-warning", "Trigger" }
                }
                if entity.is_kill_target {
                    span { class: "tag tag-success", "Kill Target" }
                }
                // Show HP overlay tag when behavior differs from is_boss default
                {
                    let shows_hp = entity.show_on_hp_overlay.unwrap_or(entity.is_boss);
                    if shows_hp && !entity.is_boss {
                        rsx! { span { class: "tag tag-info", "HP Overlay" } }
                    } else if !shows_hp && entity.is_boss {
                        rsx! { span { class: "tag tag-muted", "HP Hidden" } }
                    } else {
                        rsx! {}
                    }
                }
            }

            // Expanded content
            if expanded {
                {
                    let all_entities_for_save = all_entities.clone();
                    let all_entities_for_delete = all_entities.clone();
                    let boss_id_save = boss_id.clone();
                    let file_path_save = file_path.clone();
                    let boss_id_delete = boss_id.clone();
                    let file_path_delete = file_path.clone();

                    rsx! {
                        div { class: "list-item-body",
                            EntityEditForm {
                                entity: entity.clone(),
                                on_dirty: move |dirty: bool| is_dirty.set(dirty),
                                on_save: move |(updated, original_name): (EntityDefinition, String)| {
                                    let all = all_entities_for_save.clone();
                                    let boss_id = boss_id_save.clone();
                                    let file_path = file_path_save.clone();
                                    let item = EncounterItem::Entity(updated.clone());
                                    // Entity uses name as ID, so pass original_name for lookup
                                    let orig_id = if original_name != updated.name { Some(original_name.clone()) } else { None };
                                    // Update parent state synchronously so props refresh and dirty indicator clears
                                    let new_list: Vec<_> = all.iter()
                                        .map(|e| if e.name == original_name { updated.clone() } else { e.clone() })
                                        .collect();
                                    on_change.call(new_list);
                                    on_status.call(("Saving...".to_string(), false));
                                    spawn(async move {
                                        match api::update_encounter_item(&boss_id, &file_path, &item, orig_id.as_deref()).await {
                                            Ok(_) => {
                                                on_status.call(("Saved".to_string(), false));
                                            }
                                            Err(_) => on_status.call(("Failed to save".to_string(), true)),
                                        }
                                    });
                                },
                                on_delete: {
                                    let all_entities = all_entities_for_delete.clone();
                                    move |entity_to_delete: EntityDefinition| {
                                        let all_entities = all_entities.clone();
                                        let boss_id = boss_id_delete.clone();
                                        let file_path = file_path_delete.clone();
                                        let entity_name = entity_to_delete.name.clone();
                                        spawn(async move {
                                            match api::delete_encounter_item("entity", &entity_name, &boss_id, &file_path).await {
                                                Ok(_) => {
                                                    let updated: Vec<_> = all_entities.iter()
                                                        .filter(|e| e.name != entity_name)
                                                        .cloned()
                                                        .collect();
                                                    on_change.call(updated);
                                                    on_collapse.call(());
                                                    on_status.call(("Deleted".to_string(), false));
                                                }
                                                Err(err) => {
                                                    on_status.call((err, true));
                                                }
                                            }
                                        });
                                    }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Entity Edit Form
// ─────────────────────────────────────────────────────────────────────────────

#[component]
fn EntityEditForm(
    entity: EntityDefinition,
    on_save: EventHandler<(EntityDefinition, String)>,
    on_delete: EventHandler<EntityDefinition>,
    #[props(default)] on_dirty: EventHandler<bool>,
) -> Element {
    let original_name = entity.name.clone();
    let entity_for_draft = entity.clone();
    let entity_for_delete = entity.clone();
    let original = entity.clone();
    let mut draft = use_signal(|| entity_for_draft);
    let mut just_saved = use_signal(|| false);

    // Reset just_saved when user makes new changes after saving
    let original_for_effect = original.clone();
    use_effect(move || {
        if draft() != original_for_effect && just_saved() {
            just_saved.set(false);
        }
    });

    let has_changes = use_memo(move || !just_saved() && draft() != original);

    // Notify parent when dirty state changes
    use_effect(move || {
        on_dirty.call(has_changes());
    });

    let handle_save = {
        let orig_name = original_name.clone();
        move |_| {
            just_saved.set(true);
            let updated = draft();
            on_save.call((updated, orig_name.clone()));
        }
    };

    let handle_delete = move |_| {
        on_delete.call(entity_for_delete.clone());
    };

    rsx! {
        div { class: "entity-edit-form",
            // ─── Name ──────────────────────────────────────────────────────────
            div { class: "form-row-hz",
                label { "Name" }
                input {
                    class: "input-inline",
                    style: "width: 200px;",
                    value: "{draft().name}",
                    oninput: move |e| {
                        let mut d = draft();
                        d.name = e.value();
                        draft.set(d);
                    }
                }
            }

            // ─── NPC IDs ───────────────────────────────────────────────────────
            div { class: "form-row-hz", style: "align-items: flex-start;",
                label { style: "padding-top: 6px;", "NPC IDs" }
                NpcIdChipEditor {
                    ids: draft().ids.clone(),
                    on_change: move |new_ids| {
                        let mut d = draft();
                        d.ids = new_ids;
                        draft.set(d);
                    }
                }
            }

            // ─── Flags ─────────────────────────────────────────────────────────
            div { class: "form-section",
                div { class: "font-bold text-sm mb-xs", "Flags" }

                div { class: "flex flex-col gap-xs",
                    label { class: "flex items-center gap-xs cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: draft().is_boss,
                            onchange: move |e| {
                                let mut d = draft();
                                d.is_boss = e.checked();
                                draft.set(d);
                            }
                        }
                        span { "Is Boss" }
                    }

                    label { class: "flex items-center gap-xs cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: draft().triggers_encounter.unwrap_or(draft().is_boss),
                            onchange: move |e| {
                                let mut d = draft();
                                d.triggers_encounter = Some(e.checked());
                                draft.set(d);
                            }
                        }
                        span { "Triggers Encounter" }
                        span { class: "text-xs text-muted", "(appearance of this target loads timers)" }
                    }

                    label { class: "flex items-center gap-xs cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: draft().is_kill_target,
                            onchange: move |e| {
                                let mut d = draft();
                                d.is_kill_target = e.checked();
                                draft.set(d);
                            }
                        }
                        span { "Is Kill Target" }
                        span { class: "text-xs text-muted", "(death of all kill targets ends encounter)" }
                    }

                    label { class: "flex items-center gap-xs cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: draft().show_on_hp_overlay.unwrap_or(draft().is_boss),
                            onchange: move |e| {
                                let mut d = draft();
                                d.show_on_hp_overlay = Some(e.checked());
                                draft.set(d);
                            }
                        }
                        span { "Show on HP Overlay" }
                        span { class: "text-xs text-muted", "(display this entity on Boss HP bar)" }
                    }
                }
            }

            // ─── HP Markers ───────────────────────────────────────────────────
            div { class: "form-section",
                div { class: "flex items-center justify-between mb-xs",
                    div { class: "font-bold text-sm", "HP Markers" }
                    button {
                        class: "btn btn-sm",
                        onclick: move |_| {
                            let mut d = draft();
                            d.hp_markers.push(crate::types::HpMarker {
                                hp_percent: 50.0,
                                label: String::new(),
                            });
                            draft.set(d);
                        },
                        "+ Add Marker"
                    }
                }
                div { class: "text-xs text-muted mb-xs", "Visual indicators on the HP bar at key thresholds" }
                for (i, marker) in draft().hp_markers.iter().cloned().enumerate() {
                    div { class: "form-row-hz", style: "align-items: center;",
                        input {
                            class: "input-inline",
                            style: "width: 60px;",
                            r#type: "number",
                            step: "1",
                            min: "0",
                            max: "100",
                            value: "{marker.hp_percent}",
                            oninput: move |e| {
                                let mut d = draft();
                                if let Ok(v) = e.value().parse::<f32>() {
                                    d.hp_markers[i].hp_percent = v;
                                    draft.set(d);
                                }
                            }
                        }
                        span { class: "text-xs text-muted", "%" }
                        input {
                            class: "input-inline",
                            style: "width: 120px;",
                            placeholder: "Label...",
                            value: "{marker.label}",
                            oninput: move |e| {
                                let mut d = draft();
                                d.hp_markers[i].label = e.value();
                                draft.set(d);
                            }
                        }
                        button {
                            class: "btn btn-danger btn-xs",
                            onclick: move |_| {
                                let mut d = draft();
                                d.hp_markers.remove(i);
                                draft.set(d);
                            },
                            "×"
                        }
                    }
                }
            }

            // ─── Shield Definitions ──────────────────────────────────────────
            div { class: "form-section",
                div { class: "flex items-center justify-between mb-xs",
                    div { class: "font-bold text-sm", "Shields" }
                    button {
                        class: "btn btn-sm",
                        onclick: move |_| {
                            let mut d = draft();
                            d.shields.push(crate::types::ShieldDefinition {
                                label: String::new(),
                                trigger_effect: 0,
                                total: 0,
                                total_16: None,
                                end_trigger_effect: 0,
                            });
                            draft.set(d);
                        },
                        "+ Add Shield"
                    }
                }
                div { class: "text-xs text-muted mb-xs", "Absorb shields shown on the HP bar overlay" }
                for (i, shield) in draft().shields.iter().cloned().enumerate() {
                    div { class: "form-section", style: "padding: 4px 8px; margin-bottom: 4px;",
                        // Header row: Label + remove button
                        div { class: "form-row-hz",
                            label { "Label" }
                            input {
                                class: "input-inline",
                                style: "width: 140px;",
                                placeholder: "Shield name...",
                                value: "{shield.label}",
                                oninput: move |e| {
                                    let mut d = draft();
                                    d.shields[i].label = e.value();
                                    draft.set(d);
                                }
                            }
                            button {
                                class: "btn btn-danger btn-xs",
                                onclick: move |_| {
                                    let mut d = draft();
                                    d.shields.remove(i);
                                    draft.set(d);
                                },
                                "×"
                            }
                        }
                        // Two-column layout: HP values | Effect IDs
                        div { class: "flex gap-md", style: "margin-top: 4px;",
                            // Left column: HP values
                            div { class: "flex-1",
                                div { class: "form-row-hz",
                                    label { "Total HP" }
                                    input {
                                        class: "input-inline text-mono",
                                        style: "width: 120px;",
                                        r#type: "number",
                                        value: "{shield.total}",
                                        oninput: move |e| {
                                            let mut d = draft();
                                            if let Ok(v) = e.value().parse::<i64>() {
                                                d.shields[i].total = v;
                                                draft.set(d);
                                            }
                                        }
                                    }
                                }
                                div { class: "form-row-hz",
                                    label { "HP (16-man)" }
                                    input {
                                        class: "input-inline text-mono",
                                        style: "width: 120px;",
                                        r#type: "number",
                                        placeholder: "Same as Total HP",
                                        value: if let Some(v) = shield.total_16 { format!("{v}") } else { String::new() },
                                        oninput: move |e| {
                                            let mut d = draft();
                                            let val = e.value();
                                            d.shields[i].total_16 = if val.is_empty() {
                                                None
                                            } else {
                                                val.parse::<i64>().ok()
                                            };
                                            draft.set(d);
                                        }
                                    }
                                }
                            }
                            // Right column: Effect IDs
                            div { class: "flex-1",
                                div { class: "form-row-hz", style: "align-items: flex-start;",
                                    label { style: "padding-top: 6px;", "Start Effect" }
                                    IdChipEditor {
                                        ids: if shield.trigger_effect != 0 { vec![shield.trigger_effect] } else { vec![] },
                                        placeholder: "Effect ID (Enter)",
                                        on_change: move |ids: Vec<u64>| {
                                            let mut d = draft();
                                            d.shields[i].trigger_effect = ids.first().copied().unwrap_or(0);
                                            draft.set(d);
                                        }
                                    }
                                }
                                div { class: "form-row-hz", style: "align-items: flex-start;",
                                    label { style: "padding-top: 6px;", "End Effect" }
                                    IdChipEditor {
                                        ids: if shield.end_trigger_effect != 0 { vec![shield.end_trigger_effect] } else { vec![] },
                                        placeholder: "Effect ID (Enter)",
                                        on_change: move |ids: Vec<u64>| {
                                            let mut d = draft();
                                            d.shields[i].end_trigger_effect = ids.first().copied().unwrap_or(0);
                                            draft.set(d);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ─── Actions ─────────────────────────────────────────────────────
            div { class: "form-actions",
                button {
                    class: if has_changes() { "btn btn-success btn-sm" } else { "btn btn-sm" },
                    disabled: !has_changes(),
                    onclick: handle_save,
                    "Save"
                }
                button {
                    class: "btn btn-danger btn-sm",
                    onclick: handle_delete,
                    "Delete"
                }
            }
        }
    }
}
