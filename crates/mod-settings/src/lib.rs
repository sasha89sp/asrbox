pub mod actions;
pub mod draw;
mod events;
pub mod i18n;
pub mod layout;
pub mod layout_backup;
pub mod layout_browser_tabs;
pub mod layout_cal;
pub mod layout_dock;
pub mod layout_general;
pub mod layout_overlay;
pub mod layout_sync;
pub mod layout_sys;
pub mod layout_tabs;
pub mod layout_types;
pub mod render;
mod render_controls;
pub mod types;

#[cfg(test)]
mod tests;

use std::time::Instant;

use asrbox_platform_api::{FluentPalette, PixelRect, SettingsInputRects, WidgetFrame};

pub use layout::{build_layout, BuildLayoutArgs, MIN_SETTINGS_HEIGHT, MIN_SETTINGS_WIDTH};
pub use render::render_settings_frame;
pub use types::{
    AboutSettingsView, AppearanceSettingsView, BackupSettingsView, BrowserTab,
    CalendarSettingsView, CalendarSourceView, DiagnosticsLiveView, DockItemView, DockSettingsView,
    FileSyncSettingsView, ModuleStatusView, OverlaySettingsView, SettingsAction, SettingsSnapshot,
    SettingsTab, SliderKind, SyncItemView, SysMonitorSettingsView, WebDavBrowserView,
    WebDavDraftView, WebDavServerView, WidgetSubTab,
};

use layout::SettingsLayout;

const THEME_ANIM_MS: f32 = 220.0;

pub struct SettingsUiState {
    pub active_tab: SettingsTab,
    pub browser_tabs: Vec<BrowserTab>,
    pub active_browser_tab: BrowserTab,
    pub selected_dock: usize,
    pub selected_dock_item: Option<u64>,
    pub selected_widget: WidgetSubTab,
    pub editing_server_dir: Option<String>,
    pub search_query: String,
    pub width: u32,
    pub height: u32,
    pub scroll_y: i32,
    pub(crate) hovered_id: Option<usize>,
    pub(crate) dragging_slider: Option<(SliderKind, PixelRect)>,
    pub(crate) last_slider_val: Option<u32>,
    pub(crate) last_dock_input_key: Option<(String, u64)>,
    pub(crate) pending_aux_prefill: Option<String>,
    pub(crate) anim_from: FluentPalette,
    pub(crate) anim_to: FluentPalette,
    pub(crate) anim_start: Option<Instant>,
    pub(crate) layout: SettingsLayout,
}

impl Default for SettingsUiState {
    fn default() -> Self {
        let pal = FluentPalette::default();
        Self {
            active_tab: SettingsTab::General,
            browser_tabs: vec![BrowserTab::Main],
            active_browser_tab: BrowserTab::Main,
            selected_dock: 0,
            selected_dock_item: None,
            selected_widget: WidgetSubTab::Calendar,
            editing_server_dir: None,
            search_query: String::new(),
            width: 880,
            height: 620,
            scroll_y: 0,
            hovered_id: None,
            dragging_slider: None,
            last_slider_val: None,
            last_dock_input_key: None,
            pending_aux_prefill: None,
            anim_from: pal,
            anim_to: pal,
            anim_start: None,
            layout: SettingsLayout::default(),
        }
    }
}

impl SettingsUiState {
    pub fn set_palette_immediate(&mut self, pal: FluentPalette) {
        self.anim_from = pal;
        self.anim_to = pal;
        self.anim_start = None;
    }

    pub fn set_target_palette(&mut self, pal: FluentPalette) -> bool {
        if self.anim_to == pal {
            return false;
        }
        self.anim_from = self.current_palette();
        self.anim_to = pal;
        self.anim_start = Some(Instant::now());
        true
    }

    pub fn is_animating(&self) -> bool {
        self.anim_start.is_some()
    }

    pub fn current_palette(&mut self) -> FluentPalette {
        let Some(start) = self.anim_start else {
            return self.anim_to;
        };
        let elapsed_ms = start.elapsed().as_secs_f32() * 1000.0;
        let t = (elapsed_ms / THEME_ANIM_MS).clamp(0.0, 1.0);
        if t >= 1.0 {
            self.anim_start = None;
            self.anim_from = self.anim_to;
            return self.anim_to;
        }
        // Smooth cubic ease-out for native Fluent feel
        let eased = 1.0 - (1.0 - t).powi(3);
        FluentPalette::lerp(self.anim_from, self.anim_to, eased)
    }

    pub fn open_browser_tab(&mut self, tab: BrowserTab) {
        if !self.browser_tabs.contains(&tab) {
            self.browser_tabs.push(tab.clone());
        }
        self.active_browser_tab = tab;
        self.scroll_y = 0;
        self.editing_server_dir = None;
    }

    pub fn close_browser_tab(&mut self, index: usize) {
        let Some(tab) = self.browser_tabs.get(index) else {
            return;
        };
        if !tab.closable() {
            return;
        }
        let removed = self.browser_tabs.remove(index);
        if self.active_browser_tab == removed {
            let fallback = index.saturating_sub(1);
            self.active_browser_tab = self
                .browser_tabs
                .get(fallback)
                .cloned()
                .unwrap_or(BrowserTab::Main);
            self.scroll_y = 0;
        }
    }

    pub fn reset_dock_input_sync(&mut self) {
        self.last_dock_input_key = None;
    }

    pub fn take_aux_prefill(&mut self) -> Option<String> {
        self.pending_aux_prefill.take()
    }

    pub fn take_dock_input_sync(&mut self, snap: &SettingsSnapshot) -> Option<String> {
        if !matches!(self.active_browser_tab, BrowserTab::Main)
            || self.active_tab != SettingsTab::Dock
        {
            self.last_dock_input_key = None;
            return None;
        }
        if !snap.docks.is_empty() && self.selected_dock >= snap.docks.len() {
            self.selected_dock = snap.docks.len() - 1;
        }
        let dock = snap.docks.get(self.selected_dock)?;
        let item = self
            .selected_dock_item
            .and_then(|id| dock.items.iter().find(|i| i.id == id))
            .or_else(|| dock.items.first());
        let Some(item) = item else {
            self.selected_dock_item = None;
            self.last_dock_input_key = None;
            return None;
        };
        self.selected_dock_item = Some(item.id);
        let key = (dock.id.clone(), item.id);
        if self.last_dock_input_key.as_ref() == Some(&key) {
            return None;
        }
        self.last_dock_input_key = Some(key);
        Some(item.label.clone())
    }

    pub fn render(&mut self, snap: &SettingsSnapshot) -> (WidgetFrame, SettingsInputRects) {
        if !snap.docks.is_empty() && self.selected_dock >= snap.docks.len() {
            self.selected_dock = snap.docks.len() - 1;
        }
        self.rebuild_layout(snap);
        let max_scroll = (self.layout.content_height as i32 - self.height as i32).max(0);
        if self.scroll_y > max_scroll {
            self.scroll_y = max_scroll;
            self.rebuild_layout(snap);
        }
        let pal = self.current_palette();
        render_settings_frame(
            &self.layout,
            self.active_tab,
            self.selected_dock,
            self.hovered_id,
            &pal,
            snap.appearance.language,
        )
    }

    fn rebuild_layout(&mut self, snap: &SettingsSnapshot) {
        self.layout = build_layout(&BuildLayoutArgs {
            snap,
            tab: self.active_tab,
            browser_tabs: &self.browser_tabs,
            active_browser: &self.active_browser_tab,
            selected_dock: self.selected_dock,
            selected_dock_item: self.selected_dock_item,
            selected_widget: self.selected_widget,
            editing_server_dir: self.editing_server_dir.as_deref(),
            search_query: &self.search_query,
            width: self.width,
            height: self.height,
            scroll_y: self.scroll_y,
        });
    }
}
