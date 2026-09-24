//! Static game-data lookups exposed to the frontend.

use baras_core::game_data::MIRROR_ABILITIES;
use baras_types::MirrorAbility;

/// Republic ⇄ Imperial mirror pairs. Fetched once and cached by the frontend
/// to display mirror-class names in the Data Explorer. Returned as a list
/// because integer map keys don't survive the JSON → JS object round trip.
#[tauri::command]
pub fn get_mirror_abilities() -> Vec<MirrorAbility> {
    MIRROR_ABILITIES
        .entries()
        .map(|(id, m)| MirrorAbility {
            id: *id,
            mirror_id: m.mirror_id,
            mirror_name: m.mirror_name.to_string(),
            is_imperial: m.is_imperial,
        })
        .collect()
}
