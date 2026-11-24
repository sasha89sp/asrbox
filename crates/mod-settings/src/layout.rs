use asrbox_platform_api::{AppLanguage, PixelRect};

use crate::i18n::tr;
use crate::layout_browser_tabs::build_browser_tab;
use crate::layout_cal::build_calendar_tab;
use crate::layout_dock::build_dock_tab;
use crate::layout_general::build_general_tab;
use crate::layout_overlay::build_overlay_tab;
use crate::layout_sys::build_sysmon_tab;
use crate::layout_tabs::build_modules_tab;
use crate::types::{BrowserTab, SettingsAction, SettingsTab, SliderKind};

pub use crate::layout_types::{
    rect_contains, BuildLayoutArgs, ButtonTone, CardBlock, CardRow, ControlItem, HitKind,
    SettingsLayout, HEADER_H, MIN_SETTINGS_HEIGHT, MIN_SETTINGS_WIDTH, SIDEBAR_WIDTH, TAB_STRIP_H,
};

pub fn build_layout(args: &BuildLayoutArgs<'_>) -> SettingsLayout {
    let w = args.width.max(MIN_SETTINGS_WIDTH);
    let h = args.height.max(MIN_SETTINGS_HEIGHT);
    let has_strip = args.browser_tabs.len() > 1;
    let top_y = HEADER_H + if has_strip { TAB_STRIP_H } else { 0 };
    let mut b = LayoutBuilder::new(
        w,
        h,
        top_y,
        has_strip,
        args.scroll_y,
        args.snap.appearance.language,
        args.search_query,
    );
    b.add_sidebar_tabs();
    if has_strip {
        b.add_browser_tab_strip(args.browser_tabs, args.active_browser);
    }
    if matches!(args.active_browser, BrowserTab::Main) {
        b.add_header_search_and_reset(args.tab);
        match args.tab {
            SettingsTab::General => build_general_tab(&mut b, args.snap, args.editing_server_dir),
            SettingsTab::Modules => build_modules_tab(&mut b, args.snap),
            SettingsTab::Dock => build_dock_tab(
                &mut b,
                args.snap,
                args.selected_dock,
                args.selected_dock_item,
            ),
            SettingsTab::Calendar => build_calendar_tab(&mut b, args.snap),
            SettingsTab::SysMonitor => build_sysmon_tab(&mut b, args.snap),
            SettingsTab::Overlay => build_overlay_tab(&mut b, args.snap),
        }
    } else {
        build_browser_tab(&mut b, args.snap, args.active_browser);
    }
    b.finish()
}

pub(crate) struct LayoutBuilder {
    pub out: SettingsLayout,
    pub lang: AppLanguage,
    pub search_lower: String,
    scroll_y: i32,
    pub content_x: i32,
    pub content_w: u32,
    pub cursor_y: i32,
    next_id: usize,
}

impl LayoutBuilder {
    fn new(
        width: u32,
        height: u32,
        content_top_y: i32,
        has_tab_strip: bool,
        scroll_y: i32,
        lang: AppLanguage,
        search_query: &str,
    ) -> Self {
        let content_x = SIDEBAR_WIDTH as i32 + 24;
        let content_w = width.saturating_sub(SIDEBAR_WIDTH + 48).max(460);
        Self {
            out: SettingsLayout {
                width,
                height,
                content_top_y,
                has_tab_strip,
                ..Default::default()
            },
            lang,
            search_lower: search_query.trim().to_lowercase(),
            scroll_y: scroll_y.max(0),
            content_x,
            content_w,
            cursor_y: content_top_y + 16,
            next_id: 1,
        }
    }

    pub(crate) fn matches_search(&self, haystack: &str) -> bool {
        if self.search_lower.is_empty() {
            return true;
        }
        haystack.to_lowercase().contains(&self.search_lower)
    }

    fn add_sidebar_tabs(&mut self) {
        let mut y = 76;
        for tab in SettingsTab::ALL {
            self.push_control(
                PixelRect {
                    x: 10,
                    y,
                    width: SIDEBAR_WIDTH - 20,
                    height: 36,
                },
                HitKind::Tab(tab),
            );
            y += 40;
        }
    }

    fn add_browser_tab_strip(&mut self, tabs: &[BrowserTab], active: &BrowserTab) {
        let mut x = SIDEBAR_WIDTH as i32 + 12;
        let max_x = self.out.width as i32 - 12;
        for (index, tab) in tabs.iter().enumerate() {
            let label = tab.title(self.lang);
            let is_active = tab == active;
            let closable = tab.closable();
            let base_w =
                (label.chars().count() as u32 * 7 + if closable { 44 } else { 28 }).clamp(112, 220);
            if x + base_w as i32 > max_x {
                break;
            }
            let rect = PixelRect {
                x,
                y: 5,
                width: base_w,
                height: 28,
            };
            self.push_control(
                rect,
                HitKind::BrowserTabSelect {
                    index,
                    label,
                    active: is_active,
                    closable,
                },
            );
            if closable {
                self.push_control(
                    PixelRect {
                        x: x + base_w as i32 - 24,
                        y: 9,
                        width: 20,
                        height: 20,
                    },
                    HitKind::BrowserTabClose { index },
                );
            }
            x += base_w as i32 + 6;
        }
    }

    fn add_header_search_and_reset(&mut self, tab: SettingsTab) {
        let hy = if self.out.has_tab_strip {
            TAB_STRIP_H
        } else {
            0
        };
        let reset_w = 88u32;
        let search_w = 186u32;
        let right_edge = self.out.width as i32 - 24;
        let reset_x = right_edge - reset_w as i32;
        let search_x = reset_x - search_w as i32 - 10;
        let ctrl_y = hy + 22;
        let search_r = PixelRect {
            x: search_x,
            y: ctrl_y,
            width: search_w,
            height: 28,
        };
        self.out.search_rect = Some(search_r);
        self.out.input_rects.search = Some(PixelRect {
            x: search_r.x + 26,
            y: search_r.y + 2,
            width: search_r.width.saturating_sub(32),
            height: 24,
        });
        self.push_control(
            PixelRect {
                x: reset_x,
                y: ctrl_y,
                width: reset_w,
                height: 28,
            },
            HitKind::HeaderButton {
                label: tr(self.lang, "Сбросить", "Reset").to_owned(),
                action: SettingsAction::ResetSection(tab),
            },
        );
    }

    pub(crate) fn set_header(&mut self, title: &str, subtitle: &str) {
        self.out.header_title = title.to_owned();
        self.out.header_subtitle = subtitle.to_owned();
    }

    pub(crate) fn push_control(&mut self, rect: PixelRect, kind: HitKind) {
        let id = self.next_id;
        self.next_id += 1;
        self.out.controls.push(ControlItem { id, rect, kind });
    }

    pub(crate) fn sy(&self, logical_y: i32) -> i32 {
        logical_y - self.scroll_y
    }

    pub(crate) fn add_card(&mut self, title: Option<&str>, amber: bool, rows: Vec<CardRow>) -> i32 {
        let top = self.cursor_y;
        let body_h: u32 = rows.iter().map(|r| r.height).sum();
        let header_h = if title.is_some() { 26 } else { 0 };
        let rect = PixelRect {
            x: self.content_x,
            y: self.sy(top + header_h),
            width: self.content_w,
            height: body_h.max(44),
        };
        let shifted_rows = rows
            .into_iter()
            .map(|mut r| {
                r.y = self.sy(top + header_h + r.y);
                r
            })
            .collect();
        self.out.cards.push(CardBlock {
            rect,
            amber,
            title: title.map(str::to_owned),
            rows: shifted_rows,
        });
        self.cursor_y = top + header_h + body_h as i32 + 16;
        top + header_h
    }

    pub(crate) fn add_segmented_row(
        &mut self,
        row_y_logical: i32,
        row_h: u32,
        items: Vec<(&str, bool, SettingsAction)>,
    ) {
        let count = items.len().max(1) as u32;
        let total_w = (count * 84)
            .min(self.content_w.saturating_sub(190))
            .max(152);
        let seg_w = total_w / count;
        let start_x = self.content_x + self.content_w as i32 - total_w as i32 - 14;
        let y = self.sy(row_y_logical + (row_h as i32 - 28) / 2);
        for (i, (label, active, action)) in items.into_iter().enumerate() {
            self.push_control(
                PixelRect {
                    x: start_x + (i as i32) * (seg_w as i32),
                    y,
                    width: seg_w.saturating_sub(3),
                    height: 28,
                },
                HitKind::Segment {
                    label: label.to_owned(),
                    active,
                    action,
                },
            );
        }
    }

    pub(crate) fn push_toggle(&mut self, x: i32, y_log: i32, on: bool, action: SettingsAction) {
        let r = PixelRect {
            x,
            y: self.sy(y_log),
            width: 44,
            height: 22,
        };
        self.push_control(r, HitKind::Toggle { on, action });
    }

    pub(crate) fn push_slider(&mut self, x: i32, y_log: i32, kind: SliderKind, value: u32) {
        let (min, max) = kind.range();
        let r = PixelRect {
            x,
            y: self.sy(y_log),
            width: 150,
            height: 24,
        };
        self.push_control(
            r,
            HitKind::Slider {
                kind,
                value,
                min,
                max,
            },
        );
    }

    pub(crate) fn push_btn(
        &mut self,
        bounds: (i32, i32, u32, u32),
        label: &str,
        tone: ButtonTone,
        action: SettingsAction,
    ) {
        let (x, y_log, width, height) = bounds;
        let r = PixelRect {
            x,
            y: self.sy(y_log),
            width,
            height,
        };
        self.push_control(
            r,
            HitKind::Button {
                label: label.to_owned(),
                tone,
                action,
            },
        );
    }

    pub(crate) fn push_chip(
        &mut self,
        bounds: (i32, i32, u32, u32),
        label: &str,
        active: bool,
        action: SettingsAction,
    ) {
        let (x, y_log, width, height) = bounds;
        let r = PixelRect {
            x,
            y: self.sy(y_log),
            width,
            height,
        };
        self.push_control(
            r,
            HitKind::Chip {
                label: label.to_owned(),
                active,
                action,
            },
        );
    }

    fn finish(mut self) -> SettingsLayout {
        self.out.content_height = (self.cursor_y + 24).max(0) as u32;
        self.out
    }
}
