use asrbox_platform_api::{
    draw_asr_glass_logo, BgraFrame, FluentPalette, NotesInputRects, PixelRect, TextAlign,
    WidgetFrame, WidgetTextCmd,
};

use crate::draw::{
    draw_resize_grip, draw_vline, fill_pill_badge, fill_rect, fill_rounded_rect, push_txt,
};
use crate::notes_model::{NotesDb, NotesWidgetConfig};
use crate::notes_panes::{draw_editor_pane, draw_list_pane};

pub const NOTES_TOP_BAR_H: i32 = 34;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotesHitTarget {
    TogglePane(u8),
    SettingsBtn,
    CloseBtn,
    AddFolderBtn,
    FolderChevron(u64),
    FolderRow(u64),
    AddNoteBtn,
    AddTaskQuickBtn,
    NoteCard(u64),
    TaskCheck(u64),
    TaskDelete(u64),
    AddTaskBtn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotesHitBox {
    pub rect: PixelRect,
    pub target: NotesHitTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineEditMode {
    None,
    NewFolder,
    RenameFolder(u64),
    NewTask,
}

pub struct NotesRenderInput<'a> {
    pub cfg: &'a NotesWidgetConfig,
    pub db: &'a NotesDb,
    pub search_query: &'a str,
    pub hovered: Option<NotesHitTarget>,
    pub list_scroll: i32,
    pub inline_mode: InlineEditMode,
    pub palette: FluentPalette,
}

pub struct NotesRenderOutput {
    pub frame: WidgetFrame,
    pub inputs: NotesInputRects,
    pub hits: Vec<NotesHitBox>,
}

pub fn render_notes_widget(inp: &NotesRenderInput<'_>) -> NotesRenderOutput {
    let w = inp.cfg.width.max(300);
    let h = inp.cfg.height.max(320);
    let (wi, hi) = (w as i32, h as i32);
    let mut bgra = BgraFrame::transparent(w, h);
    let mut texts = Vec::with_capacity(64);
    let mut hits = Vec::with_capacity(32);
    let mut inputs = NotesInputRects::default();
    let pal = &inp.palette;

    fill_rect(&mut bgra, 0, 0, wi, hi, pal.window_bg);
    draw_top_bar(&mut bgra, &mut texts, &mut hits, inp, wi);
    fill_rect(&mut bgra, 0, NOTES_TOP_BAR_H, wi, 1, pal.divider);

    let (fw, lw, ew) = compute_column_widths(wi, inp.cfg);
    let body_y = NOTES_TOP_BAR_H + 1;
    let body_h = (hi - body_y).max(60);
    let mut cur_x = 0;

    if fw > 0 {
        draw_folders_pane(
            &mut bgra,
            &mut texts,
            &mut hits,
            &mut inputs,
            inp,
            cur_x,
            body_y,
            fw,
            body_h,
        );
        cur_x += fw;
        if lw > 0 || ew > 0 {
            draw_vline(&mut bgra, cur_x, body_y, hi, pal.divider);
            cur_x += 1;
        }
    }
    if lw > 0 {
        draw_list_pane(
            &mut bgra, &mut texts, &mut hits, inp, cur_x, body_y, lw, body_h,
        );
        cur_x += lw;
        if ew > 0 {
            draw_vline(&mut bgra, cur_x, body_y, hi, pal.divider);
            cur_x += 1;
        }
    }
    if ew > 0 {
        let active_note = inp.db.note(inp.cfg.active_note_id);
        draw_editor_pane(
            &mut bgra,
            &mut texts,
            &mut hits,
            &mut inputs,
            inp,
            active_note,
            cur_x,
            body_y,
            (wi - cur_x).max(80),
            body_h,
        );
    }

    draw_resize_grip(&mut bgra, pal);
    NotesRenderOutput {
        frame: WidgetFrame { bgra, texts },
        inputs,
        hits,
    }
}

fn compute_column_widths(wi: i32, cfg: &NotesWidgetConfig) -> (i32, i32, i32) {
    match (
        cfg.show_folders_pane,
        cfg.show_list_pane,
        cfg.show_editor_pane,
    ) {
        (true, true, true) => {
            let fw = (wi * 23 / 100).clamp(138, 210);
            let lw = (wi * 30 / 100).clamp(170, 260);
            (fw, lw, (wi - fw - lw - 2).max(160))
        }
        (true, true, false) => {
            let fw = (wi * 40 / 100).clamp(140, 230);
            (fw, (wi - fw - 1).max(140), 0)
        }
        (true, false, true) => {
            let fw = (wi * 28 / 100).clamp(140, 220);
            (fw, 0, (wi - fw - 1).max(160))
        }
        (false, true, true) => {
            let lw = (wi * 36 / 100).clamp(170, 280);
            (0, lw, (wi - lw - 1).max(160))
        }
        (true, false, false) => (wi, 0, 0),
        (false, true, false) => (0, wi, 0),
        (false, false, _) => (0, 0, wi),
    }
}

fn draw_top_bar(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    hits: &mut Vec<NotesHitBox>,
    inp: &NotesRenderInput<'_>,
    w: i32,
) {
    let pal = &inp.palette;
    fill_rect(frame, 0, 0, w, NOTES_TOP_BAR_H, pal.sidebar_bg);
    push_txt(
        texts,
        (12, 8, 220, 18),
        concat!("Заметки — asrbox v", env!("CARGO_PKG_VERSION")),
        (11, true),
        pal.text_secondary,
        TextAlign::Left,
    );

    let toggles = [
        (0u8, "◧", inp.cfg.show_folders_pane, w - 166),
        (1u8, "◫", inp.cfg.show_list_pane, w - 138),
        (2u8, "▭", inp.cfg.show_editor_pane, w - 110),
    ];
    for (idx, glyph, active, bx) in toggles {
        let target = NotesHitTarget::TogglePane(idx);
        if active {
            fill_pill_badge(frame, bx, 6, 24, 22, pal);
        } else if inp.hovered == Some(target) {
            fill_rounded_rect(frame, bx, 6, 24, 22, 5.0, pal.control_hover);
        }
        let col = if active { pal.accent } else { pal.text_muted };
        push_txt(
            texts,
            (bx, 6, 24, 22),
            glyph,
            (13, false),
            col,
            TextAlign::Center,
        );
        push_hit(hits, bx, 6, 24, 22, target);
    }

    for (target, bx, glyph) in [
        (NotesHitTarget::SettingsBtn, w - 68, "⚙"),
        (NotesHitTarget::CloseBtn, w - 36, "✕"),
    ] {
        let hov = inp.hovered == Some(target);
        if hov {
            let bg = if target == NotesHitTarget::CloseBtn {
                pal.danger_bg
            } else {
                pal.control_hover
            };
            fill_rounded_rect(frame, bx, 6, 26, 22, 5.0, bg);
        }
        let col = if hov && target == NotesHitTarget::CloseBtn {
            [255, 255, 255, 255]
        } else {
            pal.text_secondary
        };
        push_txt(
            texts,
            (bx, 6, 26, 22),
            glyph,
            (12, false),
            col,
            TextAlign::Center,
        );
        push_hit(hits, bx, 6, 26, 22, target);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_folders_pane(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    hits: &mut Vec<NotesHitBox>,
    inputs: &mut NotesInputRects,
    inp: &NotesRenderInput<'_>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    let pal = &inp.palette;
    fill_rect(frame, x, y, w, h, pal.sidebar_bg);
    draw_asr_glass_logo(frame, x + 12, y + 10, 32, pal.is_dark, false);
    push_txt(
        texts,
        (x + 50, y + 10, w - 58, 16),
        "asrbox",
        (12, true),
        pal.text_primary,
        TextAlign::Left,
    );
    push_txt(
        texts,
        (x + 50, y + 26, w - 58, 14),
        "Заметки",
        (11, false),
        pal.text_muted,
        TextAlign::Left,
    );

    let btn_y = y + 52;
    let btn_bg = if inp.hovered == Some(NotesHitTarget::AddFolderBtn) {
        pal.control_hover
    } else {
        pal.control_bg
    };
    fill_rounded_rect(frame, x + 10, btn_y, w - 20, 28, 7.0, btn_bg);
    push_txt(
        texts,
        (x + 10, btn_y + 5, w - 20, 18),
        "+ Папка",
        (12, true),
        pal.text_primary,
        TextAlign::Center,
    );
    push_hit(
        hits,
        x + 10,
        btn_y,
        w - 20,
        28,
        NotesHitTarget::AddFolderBtn,
    );

    let mut cy = btn_y + 38;
    if inp.inline_mode == InlineEditMode::NewFolder {
        inputs.inline_edit = Some(PixelRect {
            x: x + 12,
            y: cy,
            width: (w - 24).max(40) as u32,
            height: 24,
        });
        cy += 30;
    }

    for (fid, depth, has_children) in inp.db.visible_folder_rows() {
        if cy + 30 > y + h - 8 {
            break;
        }
        let Some(folder) = inp.db.folders.iter().find(|f| f.id == fid) else {
            continue;
        };
        let active = inp.cfg.active_folder_id == fid;
        if active {
            fill_pill_badge(frame, x + 8, cy, w - 16, 28, pal);
        } else if inp.hovered == Some(NotesHitTarget::FolderRow(fid)) {
            fill_rounded_rect(frame, x + 8, cy, w - 16, 28, 6.0, pal.nav_hover_bg);
        }
        let indent = i32::from(depth) * 14;
        if has_children {
            let chev = if folder.expanded { "▾" } else { "▸" };
            push_txt(
                texts,
                (x + 10 + indent, cy + 5, 16, 18),
                chev,
                (11, false),
                pal.text_secondary,
                TextAlign::Center,
            );
            push_hit(
                hits,
                x + 10 + indent,
                cy,
                18,
                28,
                NotesHitTarget::FolderChevron(fid),
            );
        }
        if inp.inline_mode == InlineEditMode::RenameFolder(fid) {
            inputs.inline_edit = Some(PixelRect {
                x: x + 28 + indent,
                y: cy + 2,
                width: (w - 38 - indent).max(40) as u32,
                height: 24,
            });
        } else {
            let col = if active { pal.accent } else { pal.text_primary };
            push_txt(
                texts,
                (x + 28 + indent, cy + 5, w - 38 - indent, 18),
                &format!("📁 {}", folder.name),
                (12, active),
                col,
                TextAlign::Left,
            );
            push_hit(
                hits,
                x + 26 + indent,
                cy,
                w - 34 - indent,
                28,
                NotesHitTarget::FolderRow(fid),
            );
        }
        cy += 32;
    }
}

pub(crate) fn push_hit(
    hits: &mut Vec<NotesHitBox>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    target: NotesHitTarget,
) {
    hits.push(NotesHitBox {
        rect: PixelRect {
            x,
            y,
            width: w.max(0) as u32,
            height: h.max(0) as u32,
        },
        target,
    });
}

pub fn hit_test_notes(hits: &[NotesHitBox], x: i32, y: i32) -> Option<NotesHitTarget> {
    for hb in hits.iter().rev() {
        let r = hb.rect;
        if x >= r.x
            && y >= r.y
            && x < r.x + i32::try_from(r.width).unwrap_or(0)
            && y < r.y + i32::try_from(r.height).unwrap_or(0)
        {
            return Some(hb.target);
        }
    }
    None
}
