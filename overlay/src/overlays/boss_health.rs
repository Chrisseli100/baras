//! Boss Health Bar Overlay
//!
//! Displays real-time health bars for boss NPCs in the current encounter.
//! Supports HP threshold markers (vertical lines at key HP%) and shield bars.

use std::collections::HashMap;
use std::sync::Arc;

use baras_core::context::BossHealthConfig;
use baras_core::OverlayHealthEntry;
use tiny_skia::Color;

use super::{Overlay, OverlayConfigUpdate, OverlayData};
use crate::frame::OverlayFrame;
use crate::platform::{OverlayConfig, PlatformError};
use crate::utils::color_from_rgba;
use crate::widgets::colors;
use crate::widgets::ProgressBar;
use baras_types::formatting;

/// A single effect icon to render beneath a boss HP bar.
#[derive(Debug, Clone)]
pub struct BossEffectIcon {
    pub effect_id: u64,
    pub icon_ability_id: u64,
    pub name: String,
    pub remaining_secs: f32,
    pub total_secs: f32,
    pub color: [u8; 4],
    pub show_icon: bool,
    pub icon: Option<Arc<(u32, u32, Vec<u8>)>>,
}

impl BossEffectIcon {
    pub fn progress(&self) -> f32 {
        if self.total_secs > 0.0 {
            (self.remaining_secs / self.total_secs).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    pub fn format_time(&self, european: bool) -> String {
        formatting::format_countdown_compact(self.remaining_secs, "0", european)
    }
}

/// Data sent from service to boss health overlay
#[derive(Debug, Clone, Default)]
pub struct BossHealthData {
    /// Current boss health entries (sorted by encounter order)
    pub entries: Vec<OverlayHealthEntry>,
    /// Effect icons keyed by NPC entity id (matches OverlayHealthEntry::entity_id).
    /// Keyed by id rather than name so two NPCs that share a display name show
    /// only the effects actually applied to each one.
    pub boss_icons: HashMap<i64, Vec<BossEffectIcon>>,
    /// Force the bar to clear even when `clear_after_combat` is disabled. Sent at
    /// the start of a new encounter so a stale boss HP bar doesn't linger into the
    /// next fight (e.g. pulling trash after a boss).
    pub force_clear: bool,
}

/// Base dimensions for scaling calculations
const BASE_WIDTH: f32 = 250.0;
const BASE_HEIGHT: f32 = 100.0;

/// Base layout values (at BASE_WIDTH x BASE_HEIGHT)
const BASE_BAR_HEIGHT: f32 = 20.0;
const BASE_ENTRY_SPACING: f32 = 8.0;
const BASE_PADDING: f32 = 8.0;
const BASE_FONT_SIZE: f32 = 13.0;

fn shield_bar_color() -> Color {
    Color::from_rgba8(100, 180, 255, 200)
}

fn marker_line_color() -> Color {
    Color::from_rgba8(255, 255, 255, 180)
}

/// Neutral dark background for the gutter row beneath each bar.
fn gutter_bg() -> Color {
    Color::from_rgba(0.10, 0.10, 0.10, 0.88).unwrap_or(Color::BLACK)
}

/// Quantize compression into discrete font steps so text size stays put while
/// entries are added or removed; bar geometry still compresses smoothly.
fn font_step(compression: f32) -> f32 {
    if compression >= 1.0 {
        1.0
    } else if compression >= 0.85 {
        0.85
    } else if compression >= 0.70 {
        0.70
    } else {
        0.55
    }
}

/// Maximum number of bosses we optimize scaling for
const MAX_SUPPORTED_BOSSES: usize = 7;
/// Minimum compression factor to keep entries readable
const MIN_COMPRESSION: f32 = 0.4;

/// Boss health bar overlay
pub struct BossHealthOverlay {
    frame: OverlayFrame,
    config: BossHealthConfig,
    data: BossHealthData,
    european_number_format: bool,
    /// (current, max, shield_remaining_per_shield) per entry — used to skip re-renders
    /// when HP and shields are unchanged and no boss effects are ticking. Tracking
    /// per-shield `remaining` (not just count) lets the shield bar animate as it absorbs.
    last_hp_sig: Vec<(i32, i32, Vec<i64>)>,
    /// Total boss-effect icon count from the last frame. Forces one final re-render
    /// on the trailing edge when icons disappear, so a stale "0.0" countdown text
    /// doesn't remain on screen.
    last_icon_count: usize,
}

impl BossHealthOverlay {
    /// Create a new boss health overlay
    pub fn new(
        window_config: OverlayConfig,
        config: BossHealthConfig,
        background_alpha: u8,
    ) -> Result<Self, PlatformError> {
        let mut frame = OverlayFrame::new(window_config, BASE_WIDTH, BASE_HEIGHT)?;
        frame.set_background_alpha(background_alpha);
        frame.set_label("Boss Health");

        Ok(Self {
            frame,
            config,
            data: BossHealthData::default(),
            european_number_format: false,
            last_hp_sig: Vec::new(),
            last_icon_count: 0,
        })
    }

    /// Update the config
    pub fn set_config(&mut self, config: BossHealthConfig) {
        self.config = config;
    }

    /// Update background alpha
    pub fn set_background_alpha(&mut self, alpha: u8) {
        self.frame.set_background_alpha(alpha);
    }

    /// Update the data
    pub fn set_data(&mut self, data: BossHealthData) {
        self.data = data;
    }

    /// All text in this overlay renders bold; these wrappers keep the style
    /// and its width measurements in sync (bold glyphs are wider).
    fn draw_text_bold(&mut self, text: &str, x: f32, y: f32, font_size: f32, color: Color) {
        self.frame
            .draw_text_with_glow(text, x, y, font_size, color, true, false);
    }

    fn measure_text_bold(&mut self, text: &str, font_size: f32) -> (f32, f32) {
        self.frame.measure_text_styled(text, font_size, true, false)
    }

    /// Draw the gutter row contents: the bottom strip of the contiguous
    /// bar+gutter unit, whose space is always reserved so entries never resize
    /// as elements toggle. Hosts the shield (blue fill + remaining amount,
    /// centered), the next phase marker (left) and the current target (right);
    /// absent elements simply leave their slot empty. The unit's shared
    /// background and outline are drawn by the caller.
    fn draw_gutter_row(
        &mut self,
        marker: Option<&str>,
        shield: Option<(f32, &str)>,
        target: Option<&str>,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        font: f32,
        radius: f32,
        font_color: Color,
    ) {
        let pad_x = 4.0 * self.frame.scale_factor();
        if let Some((frac, _)) = shield {
            let sh_w = w * frac.clamp(0.0, 1.0);
            if sh_w > 0.0 {
                self.frame
                    .fill_rounded_rect(x, y, sh_w, h, radius, shield_bar_color());
            }
        }
        let text_y = y + h / 2.0 + font / 3.0;
        if let Some(marker_text) = marker {
            let display = self.truncate_text_to_width(marker_text, w * 0.4, font);
            self.draw_text_bold(&display, x + pad_x, text_y, font, font_color);
        }
        if let Some(target_text) = target {
            let display = self.truncate_text_to_width(target_text, w * 0.4, font);
            let (tw, _) = self.measure_text_bold(&display, font);
            self.draw_text_bold(&display, x + w - tw - pad_x, text_y, font, font_color);
        }
        if let Some((_, amount)) = shield {
            let (tw, _) = self.measure_text_bold(amount, font);
            self.draw_text_bold(amount, x + (w - tw) / 2.0, text_y, font, font_color);
        }
    }

    /// Boss names are cut to this many words (with a trailing …) before layout.
    const NAME_MAX_WORDS: usize = 4;

    /// Draw the bar text: boss name (left) and the HP readout (right), both
    /// vertically centered within the bar.
    ///
    /// The HP readout joins the non-empty parts with a divider ("1.23M | 12.5%",
    /// "1.23M", or "12.5%") — parts are emptied upstream by the show_hp_value /
    /// show_percent config toggles. A name too wide for the space left of the
    /// HP zone word-wraps at full size; the wrapped block stays centered as a
    /// whole. All lines share one font size; only a line that still doesn't
    /// fit (e.g. one long word) scales the whole name down and then
    /// ellipsis-truncates.
    fn draw_bottom_row(
        &mut self,
        name: &str,
        health_text: &str,
        percent_text: &str,
        bar_x: f32,
        bar_top: f32,
        bar_w: f32,
        bar_h: f32,
        name_font_size: f32,
        hp_font_size: f32,
        font_color: Color,
    ) {
        let center_y = bar_top + bar_h / 2.0;
        let hp_text_y = center_y + hp_font_size / 3.0;

        // Readout segments: bold values with a smaller regular-weight divider
        // between, sharing the same baseline.
        let divider_font_size = hp_font_size * 0.8;
        // Gap on each side of the divider — tighter than a space character.
        let divider_gap = hp_font_size * 0.1;
        let mut segments: Vec<(&str, bool, f32)> = Vec::new();
        match (health_text.is_empty(), percent_text.is_empty()) {
            (false, false) => {
                segments.push((health_text, true, hp_font_size));
                segments.push(("|", false, divider_font_size));
                segments.push((percent_text, true, hp_font_size));
            }
            (false, true) => segments.push((health_text, true, hp_font_size)),
            (true, false) => segments.push((percent_text, true, hp_font_size)),
            (true, true) => {}
        }
        let mut hp_zone_w = 0.0;
        if !segments.is_empty() {
            let widths: Vec<f32> = segments
                .iter()
                .map(|(seg, bold, size)| {
                    self.frame.measure_text_styled(seg, *size, *bold, false).0
                })
                .collect();
            let has_divider = segments.len() == 3;
            let total_w: f32 = widths.iter().sum::<f32>()
                + if has_divider { divider_gap * 2.0 } else { 0.0 };
            let mut text_x = bar_x + bar_w - total_w - 8.0 * self.frame.scale_factor();
            for ((seg, bold, size), seg_w) in segments.iter().zip(&widths) {
                let is_divider = !*bold;
                if is_divider {
                    text_x += divider_gap;
                }
                self.frame.draw_text_with_glow(
                    seg,
                    text_x,
                    hp_text_y,
                    *size,
                    font_color,
                    *bold,
                    false,
                );
                text_x += seg_w;
                if is_divider {
                    text_x += divider_gap;
                }
            }
            // Quantize the reserved zone upward so bars with near-equal
            // readouts ("3.20M | 40.0%" measures a few px wider than
            // "6.20M | 77.5%" — digits aren't equal-width) give the name the
            // same room and wrap identically; it also keeps a ticking HP
            // value from re-wrapping the name every frame.
            let step = 16.0 * self.frame.scale_factor();
            hp_zone_w = ((total_w + 12.0 * self.frame.scale_factor()) / step).ceil() * step;
        }

        let pad_x = 4.0 * self.frame.scale_factor();
        let max_name_w = (bar_w - pad_x * 2.0 - hp_zone_w).max(20.0);
        // Hanging indent applied to the wrapped continuation line.
        let indent = name_font_size * 0.5;
        let lines = self.layout_name_lines(name, max_name_w, indent, name_font_size);
        // One consistent size across lines: the largest that fits the widest
        // line (the continuation line's budget is reduced by its indent).
        let mut name_font = name_font_size;
        for (i, line) in lines.iter().enumerate() {
            let avail = if i == 0 { max_name_w } else { max_name_w - indent };
            name_font = name_font.min(self.scaled_font_for_text(line, avail, name_font_size));
        }
        // Center the whole block on the bar's vertical midpoint.
        let n = lines.len() as f32;
        for (i, line) in lines.iter().enumerate() {
            let x_off = if i == 0 { 0.0 } else { indent };
            let display = self.truncate_text_to_width(line, max_name_w - x_off, name_font);
            let line_y =
                center_y + name_font / 3.0 + (i as f32 - (n - 1.0) / 2.0) * name_font_size;
            self.draw_text_bold(&display, bar_x + pad_x + x_off, line_y, name_font, font_color);
        }
    }

    /// Lay out a boss name for the bottom row: cap it at [`Self::NAME_MAX_WORDS`]
    /// words (appending … when longer, so truncation lands on a word boundary),
    /// then split into at most two lines at the word break that minimizes the
    /// widest line — "Dread Master Brontes" becomes "Dread Master" / "Brontes",
    /// never "Dread" / "Master Brontes". The continuation line's width includes
    /// the hanging `indent` so the balance accounts for it.
    fn layout_name_lines(
        &mut self,
        name: &str,
        max_width: f32,
        indent: f32,
        font_size: f32,
    ) -> Vec<String> {
        let mut words: Vec<String> = name.split_whitespace().map(String::from).collect();
        if words.len() > Self::NAME_MAX_WORDS {
            words.truncate(Self::NAME_MAX_WORDS);
            if let Some(last) = words.last_mut() {
                last.push('…');
            }
        }
        let joined = words.join(" ");
        let (joined_w, _) = self.measure_text_bold(&joined, font_size);
        if joined_w <= max_width || words.len() < 2 {
            return vec![joined];
        }
        let mut best = (f32::INFINITY, 1);
        for i in 1..words.len() {
            let (w1, _) = self.measure_text_bold(&words[..i].join(" "), font_size);
            let (w2, _) = self.measure_text_bold(&words[i..].join(" "), font_size);
            let cost = w1.max(w2 + indent);
            if cost < best.0 {
                best = (cost, i);
            }
        }
        vec![words[..best.1].join(" "), words[best.1..].join(" ")]
    }

    /// Ellipsis-truncate `text` to fit `max_width` at `font_size` (bold metrics).
    fn truncate_text_to_width(&mut self, text: &str, max_width: f32, font_size: f32) -> String {
        let (text_w, _) = self.measure_text_bold(text, font_size);
        if text_w <= max_width {
            return text.to_string();
        }
        let chars: Vec<char> = text.chars().collect();
        let mut fit = chars.len();
        while fit > 1 {
            fit -= 1;
            let candidate: String = chars[..fit].iter().collect::<String>() + "…";
            let (cw, _) = self.measure_text_bold(&candidate, font_size);
            if cw <= max_width {
                return candidate;
            }
        }
        "…".to_string()
    }

    /// Calculate scaled font size so text fits within max_width
    fn scaled_font_for_text(&mut self, text: &str, max_width: f32, base_font_size: f32) -> f32 {
        let (text_width, _) = self.measure_text_bold(text, base_font_size);
        if text_width <= max_width {
            return base_font_size;
        }

        // Scale font proportionally to fit
        let scale = max_width / text_width;
        let min_font = base_font_size * 0.6; // Don't go below 60% of base size
        (base_font_size * scale).max(min_font)
    }

    /// True when any gutter element (target / HP markers / shield) is enabled;
    /// with all three toggled off the gutter row is not drawn at all.
    fn gutter_enabled(&self) -> bool {
        self.config.show_target || self.config.show_hp_markers || self.config.show_shield
    }

    /// Heights of the wrap row (top strip of the contiguous bar that a wrapped
    /// name grows into) and the always-reserved gutter row below the bar
    /// (shield / marker / target). Both are constant per frame, so entry
    /// heights never depend on which elements are present. The gutter height
    /// is zero when every gutter element is toggled off in the config.
    fn row_metrics(&self, bar_font_size: f32, compression: f32) -> (f32, f32) {
        let pad_y = 1.5 * self.frame.scale_factor() * compression;
        let wrap_row = bar_font_size * 0.79 + pad_y * 2.0;
        let gutter = if self.gutter_enabled() {
            bar_font_size * 0.60 + pad_y * 2.0
        } else {
            0.0
        };
        (wrap_row, gutter)
    }

    /// Per-entry height: one contiguous unit (wrap row + name/HP row + gutter
    /// row), plus the icon row when boss-effect icons are present. Everything
    /// except the icon row is constant, so toggling shield/marker/target never
    /// resizes the layout.
    fn entry_height(
        &self,
        bar_height: f32,
        icon_row_height: Option<f32>,
        wrap_row_h: f32,
        gutter_h: f32,
    ) -> f32 {
        wrap_row_h + bar_height + gutter_h + icon_row_height.unwrap_or(0.0)
    }

    /// Icon row height for a given bar height (3px gap above icons + icon size + 3px gap below).
    fn icon_row_height(bar_height: f32) -> f32 {
        bar_height * 0.72 + 6.0
    }

    /// Calculate compression factor to fit entries in available height
    fn compression_factor(&self, entries: &[OverlayHealthEntry]) -> f32 {
        let height = self.frame.height() as f32;
        let padding = self.frame.scaled(BASE_PADDING);
        let bar_height = self.frame.scaled(BASE_BAR_HEIGHT);
        let entry_spacing = self.frame.scaled(BASE_ENTRY_SPACING);
        let layout_bar_font = self.frame.scaled(BASE_FONT_SIZE) * 0.70;
        let (wrap_row_h, gutter_h) = self.row_metrics(layout_bar_font, 1.0);

        let icon_row_h = Self::icon_row_height(bar_height);

        let total_needed: f32 = padding * 2.0
            + entries
                .iter()
                .map(|e| {
                    let has_icons = self.data.boss_icons.get(&e.entity_id).is_some_and(|v| !v.is_empty());
                    self.entry_height(
                        bar_height,
                        has_icons.then_some(icon_row_h),
                        wrap_row_h,
                        gutter_h,
                    ) + entry_spacing
                })
                .sum::<f32>()
            - entry_spacing;

        if total_needed <= height {
            1.0
        } else {
            (height / total_needed).max(MIN_COMPRESSION)
        }
    }

    /// Pre-compute the total content height for all visible entries.
    fn compute_content_height(&self, entries: &[OverlayHealthEntry], compression: f32) -> f32 {
        let padding = self.frame.scaled(BASE_PADDING);
        let bar_height = self.frame.scaled(BASE_BAR_HEIGHT) * compression;
        let entry_spacing = self.frame.scaled(BASE_ENTRY_SPACING) * compression;
        let layout_bar_font =
            self.frame.scaled(BASE_FONT_SIZE) * font_step(compression) * 0.70;
        let (wrap_row_h, gutter_h) = self.row_metrics(layout_bar_font, compression);

        let icon_row_h = Self::icon_row_height(bar_height);
        let mut y = padding;

        for entry in entries {
            let has_icons = self.data.boss_icons.get(&entry.entity_id).is_some_and(|v| !v.is_empty());
            y += self.entry_height(
                bar_height,
                has_icons.then_some(icon_row_h),
                wrap_row_h,
                gutter_h,
            );
            y += entry_spacing;
        }

        // Replace the trailing entry_spacing with bottom padding
        if !entries.is_empty() {
            y = y - entry_spacing + padding;
        }

        y
    }

    /// Find the next relevant HP marker: the highest hp_percent that is <= current HP%.
    /// This is the next threshold the boss will cross as HP decreases.
    fn next_marker(entry: &OverlayHealthEntry) -> Option<(f32, &str)> {
        let current_pct = entry.percent();
        entry
            .hp_markers
            .iter()
            .filter(|m| m.hp_percent <= current_pct)
            .max_by(|a, b| {
                a.hp_percent
                    .partial_cmp(&b.hp_percent)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|m| (m.hp_percent, m.label.as_str()))
    }

    /// Render a skeleton preview when in move mode: one sample boss showing
    /// every element — name, HP readout, phase marker line, and a gutter with
    /// marker text, shield, and target — all following the live config
    /// toggles so the preview reflects the user's actual settings.
    fn render_preview(&mut self) {
        let width = self.frame.width() as f32;

        let font_scale = self.config.font_scale.clamp(0.3, 3.0);
        let padding = self.frame.scaled(BASE_PADDING);
        let bar_height = self.frame.scaled(BASE_BAR_HEIGHT);
        let bar_radius = 4.0 * self.frame.scale_factor();

        let bar_color = color_from_rgba(self.config.bar_color);
        let font_color = color_from_rgba(self.config.font_color);

        let content_width = width - padding * 2.0;
        // Rows sized from the layout font; user font_scale only grows text
        // into the fixed rows (see render()).
        let layout_bar_font = self.frame.scaled(BASE_FONT_SIZE) * 0.70;
        let (wrap_row_h, gutter_h) = self.row_metrics(layout_bar_font, 1.0);
        let bar_font_size = (layout_bar_font * font_scale).min(bar_height * 0.8);
        let gutter_font = (layout_bar_font * 0.60 * font_scale).min(gutter_h * 0.85);
        let total_bar_h = wrap_row_h + bar_height;
        let unit_h = total_bar_h + gutter_h;

        let name = "Dread Master Calphayus";
        let progress = 0.72;
        let marker_pct = 0.50;

        self.frame.begin_frame();
        let bar_top = padding;

        let health_text = if self.config.show_hp_value { "7.1M" } else { "" };
        let percent_text = if self.config.show_percent { "72.0%" } else { "" };

        // Shared background for the contiguous bar + gutter unit.
        self.frame.fill_rounded_rect(
            padding,
            bar_top,
            content_width,
            unit_h,
            bar_radius,
            gutter_bg(),
        );

        // Bar background + fill span both bar rows; text is drawn per-row below.
        ProgressBar::new("", progress)
            .with_fill_color(bar_color)
            .with_bg_color(colors::dps_bar_bg())
            .with_gradient(self.config.bar_gradient)
            .render(
                &mut self.frame,
                padding,
                bar_top,
                content_width,
                total_bar_h,
                bar_font_size,
                bar_radius,
            );

        // Sample phase HP marker line through the bar (thinner than the border).
        if self.config.show_hp_markers {
            let marker_x = padding + marker_pct * content_width;
            let line_width = 0.6 * self.frame.scale_factor();
            self.frame.fill_rect(
                marker_x - line_width / 2.0,
                bar_top,
                line_width,
                total_bar_h,
                marker_line_color(),
            );
        }

        self.draw_bottom_row(
            name,
            health_text,
            percent_text,
            padding,
            bar_top,
            content_width,
            total_bar_h,
            bar_font_size * 0.79,
            bar_font_size,
            font_color,
        );

        if gutter_h > 0.0 {
            let marker = self.config.show_hp_markers.then_some("50% Burn");
            let shield = self.config.show_shield.then_some((0.6, "150.00K"));
            let target = self.config.show_target.then_some("⌖ Tank");
            self.draw_gutter_row(
                marker,
                shield,
                target,
                padding,
                bar_top + total_bar_h,
                content_width,
                gutter_h,
                gutter_font,
                bar_radius,
                font_color,
            );
        }

        if self.config.show_border {
            let border_width = 0.8 * self.frame.scale_factor();
            let border_color = color_from_rgba(self.config.border_color);
            if gutter_h > 0.0 {
                self.frame.fill_rect(
                    padding,
                    bar_top + total_bar_h - border_width / 2.0,
                    content_width,
                    border_width,
                    border_color,
                );
            }
            self.frame.stroke_rounded_rect(
                padding,
                bar_top,
                content_width,
                unit_h,
                bar_radius,
                border_width,
                border_color,
            );
        }

        self.frame.end_frame();
    }

    /// Render the overlay
    pub fn render(&mut self) {
        if self.frame.is_in_move_mode() {
            self.render_preview();
            return;
        }

        let width = self.frame.width() as f32;

        // Filter out dead bosses (0% health) and pushed bosses (HP at/below pushes_at threshold)
        let entries: Vec<_> = self
            .data
            .entries
            .iter()
            .filter(|e| e.percent() > 0.0 && !e.is_pushed())
            .take(MAX_SUPPORTED_BOSSES)
            .cloned()
            .collect();

        // Nothing to render if no living bosses
        if entries.is_empty() {
            if self.config.dynamic_background {
                self.frame.begin_frame_with_content_height(0.0);
            } else {
                self.frame.begin_frame();
            }
            self.frame.end_frame();
            return;
        }

        // Calculate compression factor based on entries
        let compression = self.compression_factor(&entries);

        // Clamp font_scale to sensible range
        let font_scale = self.config.font_scale.clamp(0.3, 3.0);

        // Apply compression to entry-specific dimensions. Fonts use quantized
        // steps (font_step) so text stays stable while geometry compresses.
        let padding = self.frame.scaled(BASE_PADDING);
        let bar_height = self.frame.scaled(BASE_BAR_HEIGHT) * compression;
        let entry_spacing = self.frame.scaled(BASE_ENTRY_SPACING) * compression;

        let bar_color = color_from_rgba(self.config.bar_color);
        let font_color = color_from_rgba(self.config.font_color);

        let content_width = width - padding * 2.0;
        let bar_radius = 4.0 * self.frame.scale_factor() * compression;
        let icon_size = bar_height * 0.72;
        let icon_spacing = 2.0;
        let time_font_size = icon_size * 0.38;

        // Fixed per-entry skeleton: one contiguous unit of wrap row (top strip
        // a wrapped name grows into) + name/HP row + gutter row. Row heights
        // are sized from the layout font (user font_scale excluded), so
        // scaling text up fills the bar's whitespace instead of growing the
        // unit; fonts are clamped to their rows.
        let layout_bar_font = self.frame.scaled(BASE_FONT_SIZE) * font_step(compression) * 0.70;
        let (wrap_row_h, gutter_h) = self.row_metrics(layout_bar_font, compression);
        let bar_font_size = (layout_bar_font * font_scale).min(bar_height * 0.8);
        let gutter_font = (layout_bar_font * 0.60 * font_scale).min(gutter_h * 0.85);

        // Pre-compute content height, then begin frame with content-aware background
        let content_height = self.compute_content_height(&entries, compression);
        if self.config.dynamic_background {
            self.frame.begin_frame_with_content_height(content_height);
        } else {
            self.frame.begin_frame();
        }

        let mut y = padding;

        for entry in &entries {
            let progress = entry.percent() / 100.0;

            // Find the next relevant HP marker (used for line + gutter label)
            let marker = if self.config.show_hp_markers {
                Self::next_marker(entry)
            } else {
                None
            };

            // Target name (rendered below the HP bar, if present and enabled)
            let target_info = if self.config.show_target {
                entry.target_name.as_ref().map(|t| format!("⌖ {}", t))
            } else {
                None
            };

            // ── Contiguous bar: wrap row (top strip) + name/HP row ──────
            let health_text = if self.config.show_hp_value {
                formatting::format_compact(entry.current as i64, self.european_number_format)
            } else {
                String::new()
            };
            let percent_text = if self.config.show_percent {
                formatting::format_pct(entry.percent() as f64, self.european_number_format)
            } else {
                String::new()
            };

            let bar_top = y;
            let total_bar_h = wrap_row_h + bar_height;
            let unit_h = total_bar_h + gutter_h;

            // Shared background for the contiguous bar + gutter unit, so the
            // two read as one shape with no seam.
            self.frame.fill_rounded_rect(
                padding,
                bar_top,
                content_width,
                unit_h,
                bar_radius,
                gutter_bg(),
            );

            // Bar background + fill span both bar rows; text is drawn per-row below.
            ProgressBar::new("", progress)
                .with_fill_color(bar_color)
                .with_bg_color(colors::dps_bar_bg())
                .with_gradient(self.config.bar_gradient)
                .render(
                    &mut self.frame,
                    padding,
                    bar_top,
                    content_width,
                    total_bar_h,
                    bar_font_size,
                    bar_radius,
                );

            // ── HP Marker Line (vertical line through the bar) ──────────
            // Slightly thinner than the 0.8px border stroke.
            if let Some((hp_pct, _)) = marker {
                let marker_x = padding + (hp_pct / 100.0) * content_width;
                let line_width = 0.6 * self.frame.scale_factor();
                self.frame.fill_rect(
                    marker_x - line_width / 2.0,
                    bar_top,
                    line_width,
                    total_bar_h,
                    marker_line_color(),
                );
            }

            // ── Bar text: boss name (left) + HP readout (right) ─────────
            self.draw_bottom_row(
                &entry.name,
                &health_text,
                &percent_text,
                padding,
                bar_top,
                content_width,
                total_bar_h,
                bar_font_size * 0.79,
                bar_font_size,
                font_color,
            );

            y += total_bar_h;

            // ── Gutter row (reserved while enabled): shield/marker/target ──
            if gutter_h > 0.0 {
                let marker_label =
                    marker.map(|(hp_pct, label)| format!("{}% {}", hp_pct as u32, label));
                let shield_info = self
                    .config
                    .show_shield
                    .then(|| entry.active_shields.first())
                    .flatten()
                    .map(|shield| {
                        let frac = if shield.total > 0 {
                            (shield.remaining as f32 / shield.total as f32).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        (
                            frac,
                            formatting::format_compact(
                                shield.remaining,
                                self.european_number_format,
                            ),
                        )
                    });
                self.draw_gutter_row(
                    marker_label.as_deref(),
                    shield_info.as_ref().map(|(frac, amt)| (*frac, amt.as_str())),
                    target_info.as_deref(),
                    padding,
                    y,
                    content_width,
                    gutter_h,
                    gutter_font,
                    bar_radius,
                    font_color,
                );
                y += gutter_h;
            }

            // Single outline around the whole unit, over the gutter contents,
            // plus an inner border on the bar/gutter seam.
            if self.config.show_border {
                let border_width = 0.8 * self.frame.scale_factor();
                let border_color = color_from_rgba(self.config.border_color);
                if gutter_h > 0.0 {
                    self.frame.fill_rect(
                        padding,
                        bar_top + total_bar_h - border_width / 2.0,
                        content_width,
                        border_width,
                        border_color,
                    );
                }
                self.frame.stroke_rounded_rect(
                    padding,
                    bar_top,
                    content_width,
                    unit_h,
                    bar_radius,
                    border_width,
                    border_color,
                );
            }

            // ── Icon Row (below gutter, only when icons are present) ────
            let entry_icons = self.data.boss_icons.get(&entry.entity_id);
            if entry_icons.is_some_and(|v| !v.is_empty()) {
                let icons = entry_icons.unwrap();
                let icon_y = y + 3.0;
                let mut icon_x = padding;

                for icon_entry in icons {
                    let drawn = if icon_entry.show_icon {
                        if let Some(ref img) = icon_entry.icon {
                            let (iw, ih, ref rgba) = **img;
                            self.frame.draw_image(rgba, iw, ih, icon_x, icon_y, icon_size, icon_size);
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if !drawn {
                        self.frame.fill_rounded_rect(
                            icon_x, icon_y, icon_size, icon_size, 2.0,
                            color_from_rgba(icon_entry.color),
                        );
                    }

                    // Clock wipe — dark overlay from top, shrinks as time remains
                    let overlay_h = icon_size * (1.0 - icon_entry.progress());
                    if overlay_h > 1.0 {
                        self.frame.fill_rect(
                            icon_x, icon_y, icon_size, overlay_h,
                            color_from_rgba([0, 0, 0, 140]),
                        );
                    }

                    self.frame.stroke_rounded_rect(
                        icon_x, icon_y, icon_size, icon_size, 2.0, 1.0, colors::white(),
                    );

                    let time_text = icon_entry.format_time(self.european_number_format);
                    // Direct frame calls: `self.data` is borrowed by the icon
                    // loop, so the `self` bold wrappers can't be used here.
                    let (tw, _) = self.frame.measure_text_styled(&time_text, time_font_size, true, false);
                    let time_color = if icon_entry.remaining_secs <= 3.0 {
                        colors::effect_debuff()
                    } else {
                        colors::white()
                    };
                    self.frame.draw_text_with_glow(
                        &time_text,
                        icon_x + (icon_size - tw) / 2.0,
                        icon_y + icon_size / 2.0 + time_font_size * 0.4,
                        time_font_size,
                        time_color,
                        true,
                        false,
                    );

                    icon_x += icon_size + icon_spacing;
                }

                y += icon_size + 6.0;
            }

            y += entry_spacing;
        }

        // End frame (resize indicator, commit)
        self.frame.end_frame();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Overlay Trait Implementation
// ─────────────────────────────────────────────────────────────────────────────

impl Overlay for BossHealthOverlay {
    fn update_data(&mut self, data: OverlayData) -> bool {
        if let OverlayData::BossHealth(boss_data) = data {
            // When clear_after_combat is disabled, ignore empty clears so the last
            // boss health remains visible — unless force_clear is set, which marks
            // the start of a new encounter and must always wipe the stale bar.
            if boss_data.entries.is_empty()
                && !self.config.clear_after_combat
                && !boss_data.force_clear
            {
                return false;
            }

            // Active effect icons tick every frame — render every frame while present.
            // Track total count so the trailing edge (last icon expires) forces one
            // final render to erase the stale "0.0" countdown text.
            let new_icon_count: usize =
                boss_data.boss_icons.values().map(|v| v.len()).sum();
            let icons_changed = new_icon_count != self.last_icon_count;
            let has_active_effects = new_icon_count > 0;
            self.last_icon_count = new_icon_count;

            // Re-render when HP or shield state changed. Per-shield `remaining` is
            // included so absorbing damage smoothly redraws the shield bar without
            // requiring a count change.
            let new_sig: Vec<(i32, i32, Vec<i64>)> = boss_data
                .entries
                .iter()
                .map(|e| {
                    (
                        e.current,
                        e.max,
                        e.active_shields.iter().map(|s| s.remaining).collect(),
                    )
                })
                .collect();
            let hp_changed = new_sig != self.last_hp_sig;
            self.last_hp_sig = new_sig;

            self.set_data(boss_data);
            has_active_effects || icons_changed || hp_changed
        } else {
            false
        }
    }

    fn update_config(&mut self, config: OverlayConfigUpdate) {
        if let OverlayConfigUpdate::BossHealth(boss_config, alpha, european) = config {
            self.set_config(boss_config);
            self.set_background_alpha(alpha);
            self.european_number_format = european;
        }
    }

    fn render(&mut self) {
        BossHealthOverlay::render(self);
    }

    fn poll_events(&mut self) -> bool {
        self.frame.poll_events()
    }

    fn frame(&self) -> &OverlayFrame {
        &self.frame
    }

    fn frame_mut(&mut self) -> &mut OverlayFrame {
        &mut self.frame
    }
}
