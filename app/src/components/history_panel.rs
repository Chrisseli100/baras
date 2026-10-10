//! Historical tab: recorded boss pulls and dummy parses across every log file,
//! organised as an Operation → Boss tree with a per-boss pull list.
//!
//! The backend keeps the index; this panel only ever holds the tree aggregates
//! and the rows for the selected boss.

use dioxus::prelude::*;
use wasm_bindgen::prelude::*;

use baras_types::formatting;
use baras_types::history::{BackfillEvent, BossNode, PullFilter, PullRow, PullSortKey as SortKey};

use crate::api;
use crate::components::class_icons::{get_class_icon, get_role_icon};
use crate::components::{ToastSeverity, use_toast};
use crate::types::UiSessionState;

#[derive(Props, Clone, PartialEq)]
pub struct HistoryPanelProps {
    pub european: bool,
    /// Filters and selection persist here across tab switches
    pub state: Signal<UiSessionState>,
    pub is_live_tailing: Signal<bool>,
}

const ROLES: [(&str, &str); 3] = [("Tank", "Tank"), ("Healer", "Healer"), ("Dps", "DPS")];
const TIERS: [&str; 3] = ["Story", "Veteran", "Master"];
/// Quick date-range presets: label and days back (None = start of this year)
const DATE_PRESETS: [(&str, Option<i32>); 4] = [("7d", Some(7)), ("30d", Some(30)), ("90d", Some(90)), ("Year", None)];

/// `YYYY-MM-DD` for `days` ago in local time, or Jan 1 of this year
fn preset_start(days: Option<i32>) -> String {
    const DAY_MS: f64 = 86_400_000.0;
    let d = js_sys::Date::new(&JsValue::from_f64(
        js_sys::Date::now() - days.unwrap_or(0) as f64 * DAY_MS,
    ));
    match days {
        Some(_) => format!("{:04}-{:02}-{:02}", d.get_full_year(), d.get_month() + 1, d.get_date()),
        None => format!("{:04}-01-01", d.get_full_year()),
    }
}

/// Tree selection: (operation, boss)
type BossKey = (String, String);

fn short_ts(iso: &str) -> String {
    iso.get(..16).unwrap_or(iso).replacen('T', " ", 1)
}

/// The boss to show: the user's pick if it still exists, else the first boss
fn effective_selection(selected: Option<BossKey>, bosses: &[BossNode]) -> Option<BossKey> {
    selected
        .filter(|(op, boss)| bosses.iter().any(|n| n.operation == *op && n.boss == *boss))
        .or_else(|| bosses.first().map(|n| (n.operation.clone(), n.boss.clone())))
}

/// Group consecutive nodes (already sorted by operation) into tree sections
fn group_by_operation<'a>(bosses: impl Iterator<Item = &'a BossNode>) -> Vec<(&'a str, Vec<&'a BossNode>)> {
    let mut tree: Vec<(&'a str, Vec<&'a BossNode>)> = Vec::new();
    for node in bosses {
        match tree.last_mut() {
            Some((op, nodes)) if *op == node.operation => nodes.push(node),
            _ => tree.push((&node.operation, vec![node])),
        }
    }
    tree
}

#[component]
pub fn HistoryPanel(props: HistoryPanelProps) -> Element {
    let eu = props.european;
    let mut is_live_tailing = props.is_live_tailing;
    let mut toast = use_toast();

    // Initialised from the cached state, synced back one way (see data_explorer.rs)
    let mut filter = use_signal(|| props.state.read().history.filter.clone());
    let mut difficulty_filter = use_signal(|| props.state.read().history.difficulty.clone());
    let mut selected = use_signal(|| props.state.read().history.selected.clone());
    let mut expanded = use_signal(|| props.state.read().history.expanded.clone());
    let mut boss_search = use_signal(|| props.state.read().history.boss_search.clone());
    let mut sort_key = use_signal(|| props.state.read().history.sort_key);
    let mut sort_desc = use_signal(|| props.state.read().history.sort_desc);
    let mut state = props.state;
    use_effect(move || {
        let mut s = state.write();
        s.history.filter = filter.read().clone();
        s.history.difficulty = difficulty_filter.read().clone();
        s.history.selected = selected.read().clone();
        s.history.expanded = expanded.read().clone();
        s.history.boss_search = boss_search.read().clone();
        s.history.sort_key = *sort_key.read();
        s.history.sort_desc = *sort_desc.read();
    });
    let mut set_filter = move |update: fn(&mut PullFilter, String), value: String| {
        update(&mut filter.write(), value);
    };

    let mut overview = use_resource(move || {
        let f = filter.read().clone();
        async move { api::get_pull_history_overview(&f).await.unwrap_or_default() }
    });

    // Backfill progress relayed from the worker subprocess; None = idle.
    // The Tauri callback runs outside the Dioxus runtime, so it only writes
    // this signal; anything that spawns (resource restart, toast timers)
    // happens in the effect below, inside the runtime.
    let mut backfill = use_signal(|| None::<BackfillEvent>);
    use_future(move || async move {
        let closure = Closure::new(move |event: JsValue| {
            let Ok(payload) = js_sys::Reflect::get(&event, &JsValue::from_str("payload")) else {
                return;
            };
            if let Ok(ev) = serde_wasm_bindgen::from_value::<BackfillEvent>(payload) {
                let _ = backfill.try_write().map(|mut w| *w = Some(ev));
            }
        });
        api::tauri_listen("history-backfill", &closure).await;
        closure.forget();
    });
    use_effect(move || {
        if let Some(BackfillEvent::Done { pulls, logs, failed, .. }) = *backfill.read() {
            toast.show(
                format!("Backfill complete: {pulls} pulls across {logs} logs, {failed} failed"),
                ToastSeverity::Normal,
            );
            overview.restart();
        }
    });
    let backfill_running = matches!(*backfill.read(), Some(BackfillEvent::Progress { .. }));

    let overview_read = overview.read();
    let loading = overview_read.is_none();
    let (characters, disciplines, bosses): (&[String], &[String], &[BossNode]) = overview_read
        .as_ref()
        .map(|o| (o.characters.as_slice(), o.disciplines.as_slice(), o.bosses.as_slice()))
        .unwrap_or((&[], &[], &[]));
    let search = boss_search.read().to_lowercase();
    let tree = group_by_operation(bosses.iter().filter(|n| {
        search.is_empty() || n.boss.to_lowercase().contains(&search) || n.operation.to_lowercase().contains(&search)
    }));

    let current = effective_selection(selected.read().clone(), bosses);
    let current_node = current
        .as_ref()
        .and_then(|(op, boss)| bosses.iter().find(|n| n.operation == *op && n.boss == *boss));

    // Reads signals (not render-local values) so the resource re-runs on change
    let pulls = use_resource(move || {
        let key = overview
            .read()
            .as_ref()
            .and_then(|o| effective_selection(selected.read().clone(), &o.bosses));
        let f = filter.read().clone();
        async move {
            let (op, boss) = key?;
            api::get_boss_pulls(&op, &boss, &f).await
        }
    });

    let pulls_read = pulls.read();
    let rows: &[PullRow] = pulls_read.as_ref().and_then(|p| p.as_deref()).unwrap_or(&[]);
    let mut difficulties: Vec<&str> = rows.iter().map(|r| r.difficulty.as_str()).collect();
    difficulties.sort();
    difficulties.dedup();
    let df = difficulty_filter.read().clone();
    // A stale difficulty from a previous boss falls back to All
    let df = if difficulties.contains(&df.as_str()) { df } else { String::new() };

    let key = *sort_key.read();
    let mut visible: Vec<&PullRow> = rows
        .iter()
        .filter(|r| df.is_empty() || r.difficulty == df)
        .collect();
    visible.sort_by(|a, b| {
        let ord = match key {
            SortKey::Date => a.timestamp.cmp(&b.timestamp),
            SortKey::Duration => a.duration_seconds.cmp(&b.duration_seconds),
            SortKey::Dps => a.dps.cmp(&b.dps),
            SortKey::Hps => a.hps.cmp(&b.hps),
        };
        if *sort_desc.read() { ord.reverse() } else { ord }
    });

    let mut on_sort = move |key: SortKey| {
        if *sort_key.read() == key {
            sort_desc.toggle();
        } else {
            sort_key.set(key);
            sort_desc.set(true);
        }
    };
    let sort_class = move |k: SortKey| {
        if *sort_key.read() != k {
            "sortable-header"
        } else if *sort_desc.read() {
            "sortable-header sorted desc"
        } else {
            "sortable-header sorted asc"
        }
    };
    let fmt_opt = move |n: Option<i64>| n.map(|v| formatting::format_compact(v, eu)).unwrap_or_else(|| "-".into());
    let f = filter.read().clone();

    rsx! {
        div { class: "pull-history",
            div { class: "pull-history-toolbar",
                select {
                    value: "{f.character}",
                    onchange: move |e| set_filter(|f, v| f.character = v, e.value()),
                    option { value: "", "All characters" }
                    for c in characters.iter() {
                        option { value: "{c}", "{c}" }
                    }
                }
                select {
                    value: "{f.role}",
                    onchange: move |e| set_filter(|f, v| f.role = v, e.value()),
                    option { value: "", "All roles" }
                    for (value, label) in ROLES {
                        option { value: "{value}", "{label}" }
                    }
                }
                select {
                    value: "{f.discipline}",
                    onchange: move |e| set_filter(|f, v| f.discipline = v, e.value()),
                    option { value: "", "All disciplines" }
                    for d in disciplines.iter() {
                        option { value: "{d}", "{d}" }
                    }
                }
                select {
                    value: "{f.tier}",
                    onchange: move |e| set_filter(|f, v| f.tier = v, e.value()),
                    option { value: "", "All difficulties" }
                    for t in TIERS {
                        option { value: "{t}", "{t}" }
                    }
                }
                label { class: "pull-history-toggle",
                    input {
                        r#type: "checkbox",
                        checked: f.operations_only,
                        onchange: move |e| filter.write().operations_only = e.checked(),
                    }
                    " Operations only"
                }
                label { class: "pull-history-toggle",
                    input {
                        r#type: "checkbox",
                        checked: f.kills_only,
                        onchange: move |e| filter.write().kills_only = e.checked(),
                    }
                    " Kills only"
                }
                div { class: "pull-history-dates",
                    div { class: "pull-history-result-tabs",
                        button {
                            class: if f.date_from.is_empty() && f.date_to.is_empty() { "filter-tab active" } else { "filter-tab" },
                            onclick: move |_| {
                                let mut w = filter.write();
                                w.date_from.clear();
                                w.date_to.clear();
                            },
                            "All"
                        }
                        for (label, days) in DATE_PRESETS {
                            {
                                let start = preset_start(days);
                                let active = f.date_to.is_empty() && f.date_from == start;
                                rsx! {
                                    button {
                                        class: if active { "filter-tab active" } else { "filter-tab" },
                                        onclick: move |_| {
                                            let mut w = filter.write();
                                            w.date_from = start.clone();
                                            w.date_to.clear();
                                        },
                                        "{label}"
                                    }
                                }
                            }
                        }
                    }
                    input {
                        r#type: "text",
                        class: "pull-history-date",
                        placeholder: "from YYYY-MM-DD",
                        maxlength: "10",
                        value: "{f.date_from}",
                        onchange: move |e| set_filter(|f, v| f.date_from = v, e.value()),
                    }
                    span { "–" }
                    input {
                        r#type: "text",
                        class: "pull-history-date",
                        placeholder: "to YYYY-MM-DD",
                        maxlength: "10",
                        value: "{f.date_to}",
                        onchange: move |e| set_filter(|f, v| f.date_to = v, e.value()),
                    }
                    if !f.date_from.is_empty() || !f.date_to.is_empty() {
                        button {
                            class: "btn btn-xs btn-ghost",
                            title: "Clear date range",
                            onclick: move |_| {
                                let mut w = filter.write();
                                w.date_from.clear();
                                w.date_to.clear();
                            },
                            i { class: "fa-solid fa-xmark" }
                        }
                    }
                }
                span { class: "pull-history-count",
                    if let Some(BackfillEvent::Progress { current, total, .. }) = *backfill.read() {
                        "Backfilling {current}/{total}..."
                    } else if loading {
                        "Loading..."
                    } else {
                        "{bosses.iter().map(|n| n.pulls).sum::<u32>()} recorded pulls"
                    }
                }
                button {
                    class: "btn btn-sm btn-ghost",
                    title: "Re-scan every log in the log directory and record its pulls",
                    disabled: backfill_running,
                    onclick: move |_| {
                        spawn(async move {
                            match api::start_history_backfill().await {
                                Ok(()) => backfill.set(Some(BackfillEvent::Progress { current: 0, total: 0, file: String::new(), pulls: 0 })),
                                Err(err) => toast.show(format!("Backfill failed to start: {err}"), ToastSeverity::Normal),
                            }
                        });
                    },
                    i { class: if backfill_running { "fa-solid fa-spinner fa-spin" } else { "fa-solid fa-database" } }
                    " Backfill"
                }
                button {
                    class: "btn btn-sm btn-ghost",
                    title: "Reload history",
                    onclick: move |_| overview.restart(),
                    i { class: "fa-solid fa-rotate" }
                }
            }

            div { class: "pull-history-body",
                aside { class: "explorer-sidebar",
                    div { class: "sidebar-search",
                        input {
                            class: "boss-search-input",
                            r#type: "text",
                            placeholder: "Filter bosses...",
                            value: "{boss_search}",
                            oninput: move |e| boss_search.set(e.value()),
                        }
                    }
                    div { class: "sidebar-encounter-list",
                        if !loading && tree.is_empty() {
                            div { class: "sidebar-empty",
                                i { class: "fa-solid fa-clock-rotate-left" }
                                p { "No recorded pulls" }
                                p { class: "hint", "Boss pulls and dummy parses are recorded as log files are parsed." }
                            }
                        }
                        for (op, nodes) in tree.iter() {
                            {
                                let op_key = op.to_string();
                                let is_collapsed = search.is_empty() && !expanded.read().contains(*op);
                                let chevron = if is_collapsed { "fa-chevron-right" } else { "fa-chevron-down" };
                                let op_pulls: u32 = nodes.iter().map(|n| n.pulls).sum();
                                rsx! {
                                    div {
                                        class: "sidebar-section-header",
                                        onclick: move |_| {
                                            let mut set = expanded.write();
                                            if !set.remove(&op_key) {
                                                set.insert(op_key.clone());
                                            }
                                        },
                                        i { class: "fa-solid {chevron} collapse-icon" }
                                        span { class: "section-area", "{op}" }
                                        span { class: "section-count", "{op_pulls}" }
                                    }
                                    if !is_collapsed {
                                        for node in nodes.iter() {
                                            {
                                                let key = (node.operation.clone(), node.boss.clone());
                                                let is_selected = current.as_ref() == Some(&key);
                                                rsx! {
                                                    div {
                                                        class: if is_selected { "sidebar-encounter-item selected" } else { "sidebar-encounter-item" },
                                                        onclick: move |_| selected.set(Some(key.clone())),
                                                        div { class: "encounter-main",
                                                            span { class: "encounter-name", "{node.boss}" }
                                                            span { class: "pull-tree-count", title: "pulls / kills", "{node.pulls}/{node.kills}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                section { class: "pull-history-main",
                    if let Some(node) = current_node {
                        div { class: "pull-history-header",
                            div { class: "pull-history-title",
                                h3 { "{node.boss}" }
                                span { class: "pull-history-operation", "{node.operation}" }
                            }
                            div { class: "pull-history-stats",
                                span { "{node.pulls} pulls" }
                                span { "·" }
                                span { "{node.kills} kills" }
                                if node.pulls > 0 {
                                    span { class: "pull-history-rate", "({node.kills * 100 / node.pulls}%)" }
                                }
                            }
                            div { class: "pull-history-result-tabs",
                                button {
                                    class: if df.is_empty() { "filter-tab active" } else { "filter-tab" },
                                    onclick: move |_| difficulty_filter.set(String::new()),
                                    "All"
                                }
                                for d in difficulties.iter() {
                                    {
                                        let value = d.to_string();
                                        rsx! {
                                            button {
                                                class: if df == *d { "filter-tab active" } else { "filter-tab" },
                                                onclick: move |_| difficulty_filter.set(value.clone()),
                                                "{d}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "pull-history-table-wrap",
                            table { class: "pull-history-table",
                                thead {
                                    tr {
                                        th { class: sort_class(SortKey::Date), onclick: move |_| on_sort(SortKey::Date), "Date" }
                                        th { "Character" }
                                        th { "Discipline" }
                                        th { "Difficulty" }
                                        th { class: sort_class(SortKey::Duration), onclick: move |_| on_sort(SortKey::Duration), "Duration" }
                                        th { "Result" }
                                        th { class: sort_class(SortKey::Dps), onclick: move |_| on_sort(SortKey::Dps), "DPS" }
                                        th { class: sort_class(SortKey::Hps), onclick: move |_| on_sort(SortKey::Hps), "HPS" }
                                        th { "Log" }
                                    }
                                }
                                tbody {
                                    for row in visible.iter() {
                                        tr {
                                            key: "{row.filename}:{row.encounter_id}",
                                            td { {short_ts(&row.timestamp)} }
                                            td { "{row.character}" }
                                            td {
                                                // Wrapper keeps the td a real table cell (flex on a td breaks its gridline)
                                                span { class: "pull-discipline",
                                                    if let Some(icon) = row.role_icon.as_deref().and_then(get_role_icon) {
                                                        img { class: "role-icon", src: *icon, alt: "" }
                                                    }
                                                    if let Some(icon) = row.discipline_icon.as_deref().and_then(get_class_icon) {
                                                        img { class: "discipline-icon", src: *icon, alt: "" }
                                                    }
                                                    {row.discipline.clone().unwrap_or_else(|| "-".into())}
                                                }
                                            }
                                            td { "{row.difficulty}" }
                                            td { {formatting::format_duration(row.duration_seconds)} }
                                            td {
                                                span { class: if row.success { "pull-result kill" } else { "pull-result wipe" },
                                                    if row.success { "Kill" } else { "Wipe" }
                                                }
                                            }
                                            td { {fmt_opt(row.dps)} }
                                            td { {fmt_opt(row.hps)} }
                                            td {
                                                if let Some(path) = row.path.clone() {
                                                    button {
                                                        class: "btn btn-xs btn-ghost",
                                                        title: "Open this pull in the Data Explorer",
                                                        onclick: {
                                                            let encounter_id = row.encounter_id;
                                                            move |_| {
                                                                let path = path.clone();
                                                                spawn(async move {
                                                                    // Tab switch happens in app.rs on "select-encounter"
                                                                    match api::open_historical_encounter(&path, encounter_id).await {
                                                                        Ok(()) => is_live_tailing.set(false),
                                                                        Err(err) => toast.show(
                                                                            format!("Failed to open log file: {err}"),
                                                                            ToastSeverity::Normal,
                                                                        ),
                                                                    }
                                                                });
                                                            }
                                                        },
                                                        i { class: "fa-solid fa-eye" }
                                                    }
                                                } else {
                                                    span { class: "pull-log-missing", title: "Log file no longer exists", "—" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if !loading {
                        div { class: "panel-placeholder",
                            i { class: "fa-solid fa-clock-rotate-left" }
                            p { "Select a boss to see its pulls." }
                        }
                    }
                }
            }
        }
    }
}
