use asrbox_platform_api::{BgraFrame, NotesInputRects, PixelRect, TextAlign, WidgetTextCmd};

use crate::draw::{fill_pill_badge, fill_rect, fill_rounded_rect, push_txt};
use crate::notes_model::NoteDocument;
use crate::notes_render::{
    push_hit, InlineEditMode, NotesHitBox, NotesHitTarget, NotesRenderInput,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_list_pane(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    hits: &mut Vec<NotesHitBox>,
    inp: &NotesRenderInput<'_>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    let pal = &inp.palette;
    push_txt(
        texts,
        (x + 14, y + 12, w - 28, 24),
        "Заметки",
        (16, true),
        pal.text_primary,
        TextAlign::Left,
    );

    let btn_y = y + 46;
    let bw = ((w - 30) / 2).max(58);
    for (idx, (target, lbl)) in [
        (NotesHitTarget::AddNoteBtn, "+ Заметка"),
        (NotesHitTarget::AddTaskQuickBtn, "+ Задача"),
    ]
    .into_iter()
    .enumerate()
    {
        let bx = x + 12 + (idx as i32) * (bw + 6);
        let bg = if inp.hovered == Some(target) {
            pal.control_hover
        } else {
            pal.control_bg
        };
        fill_rounded_rect(frame, bx, btn_y, bw, 28, 7.0, bg);
        push_txt(
            texts,
            (bx, btn_y + 5, bw, 18),
            lbl,
            (12, false),
            pal.text_primary,
            TextAlign::Center,
        );
        push_hit(hits, bx, btn_y, bw, 28, target);
    }

    let notes = inp
        .db
        .filtered_notes(inp.cfg.active_folder_id, inp.search_query);
    let mut cy = btn_y + 38 - inp.list_scroll.max(0);
    if notes.is_empty() {
        push_txt(
            texts,
            (x + 14, btn_y + 52, w - 28, 36),
            "Нет заметок.\nНажмите «+ Заметка».",
            (12, false),
            pal.text_muted,
            TextAlign::Center,
        );
        return;
    }

    for note in notes {
        if cy + 58 < btn_y + 34 {
            cy += 64;
            continue;
        }
        if cy > y + h - 20 {
            break;
        }
        let active = inp.cfg.active_note_id == note.id;
        let hov = inp.hovered == Some(NotesHitTarget::NoteCard(note.id));
        if active {
            fill_pill_badge(frame, x + 10, cy, w - 20, 56, pal);
        } else {
            let bg = if hov { pal.control_hover } else { pal.card_bg };
            fill_rounded_rect(frame, x + 10, cy, w - 20, 56, 8.0, bg);
        }
        let title = if note.title.trim().is_empty() {
            "Без названия"
        } else {
            &note.title
        };
        push_txt(
            texts,
            (x + 18, cy + 8, w - 36, 20),
            title,
            (13, true),
            pal.text_primary,
            TextAlign::Left,
        );
        push_txt(
            texts,
            (x + 18, cy + 31, w - 92, 16),
            &note.updated_at,
            (11, false),
            pal.text_muted,
            TextAlign::Left,
        );
        let (done, total) = note.task_counts();
        if total > 0 {
            let badge = format!("☑ {done}/{total}");
            let col = if done == total {
                pal.success
            } else {
                pal.accent
            };
            push_txt(
                texts,
                (x + w - 74, cy + 31, 56, 16),
                &badge,
                (11, true),
                col,
                TextAlign::Right,
            );
        }
        push_hit(
            hits,
            x + 10,
            cy,
            w - 20,
            56,
            NotesHitTarget::NoteCard(note.id),
        );
        cy += 64;
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_editor_pane(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    hits: &mut Vec<NotesHitBox>,
    inputs: &mut NotesInputRects,
    inp: &NotesRenderInput<'_>,
    note_opt: Option<&NoteDocument>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    let pal = &inp.palette;
    inputs.search = Some(PixelRect {
        x: x + 14,
        y: y + 10,
        width: (w - 28).max(80) as u32,
        height: 26,
    });

    let Some(note) = note_opt else {
        push_txt(
            texts,
            (x + 14, y + 80, w - 28, 24),
            "Выберите или создайте заметку",
            (13, false),
            pal.text_muted,
            TextAlign::Center,
        );
        return;
    };

    inputs.title = Some(PixelRect {
        x: x + 14,
        y: y + 46,
        width: (w - 28).max(80) as u32,
        height: 28,
    });
    push_txt(
        texts,
        (x + 16, y + 78, w - 32, 16),
        &format!("Обновлено: {}", note.updated_at),
        (11, false),
        pal.text_muted,
        TextAlign::Left,
    );

    let mut cy = y + 100;
    push_txt(
        texts,
        (x + 16, cy, 120, 18),
        "Задачи:",
        (12, true),
        pal.text_primary,
        TextAlign::Left,
    );
    let add_x = x + w - 128;
    if inp.hovered == Some(NotesHitTarget::AddTaskBtn) {
        fill_rounded_rect(frame, add_x, cy - 2, 114, 22, 5.0, pal.control_hover);
    }
    push_txt(
        texts,
        (add_x, cy, 114, 18),
        "+ Добавить задачу",
        (11, true),
        pal.accent,
        TextAlign::Right,
    );
    push_hit(hits, add_x, cy - 2, 114, 22, NotesHitTarget::AddTaskBtn);
    cy += 24;

    let max_tasks_y = (y + h / 2).max(cy + 60);
    for task in &note.tasks {
        if cy + 24 > max_tasks_y {
            break;
        }
        let cx = x + 16;
        if task.done {
            fill_rounded_rect(frame, cx, cy + 2, 16, 16, 4.0, pal.accent);
            push_txt(
                texts,
                (cx, cy + 1, 16, 17),
                "✓",
                (12, true),
                pal.accent_on,
                TextAlign::Center,
            );
        } else {
            fill_rounded_rect(frame, cx, cy + 2, 16, 16, 4.0, pal.control_bg);
            fill_rounded_rect(frame, cx + 1, cy + 3, 14, 14, 3.0, pal.window_bg);
        }
        let tcol = if task.done {
            pal.text_muted
        } else {
            pal.text_primary
        };
        push_txt(
            texts,
            (cx + 24, cy + 1, w - 76, 18),
            &task.text,
            (12, false),
            tcol,
            TextAlign::Left,
        );
        if task.done {
            let line_w = ((task.text.chars().count() as i32) * 7).min(w - 86).max(20);
            fill_rect(frame, cx + 24, cy + 10, line_w, 1, pal.text_muted);
        }
        push_hit(hits, cx, cy, w - 58, 22, NotesHitTarget::TaskCheck(task.id));

        let del_x = x + w - 34;
        let del_hov = inp.hovered == Some(NotesHitTarget::TaskDelete(task.id));
        if del_hov {
            fill_rounded_rect(frame, del_x, cy, 20, 20, 4.0, pal.danger_bg);
        }
        let dcol = if del_hov {
            [255, 255, 255, 255]
        } else {
            pal.text_muted
        };
        push_txt(
            texts,
            (del_x, cy, 20, 20),
            "×",
            (12, false),
            dcol,
            TextAlign::Center,
        );
        push_hit(hits, del_x, cy, 20, 20, NotesHitTarget::TaskDelete(task.id));
        cy += 24;
    }

    if inp.inline_mode == InlineEditMode::NewTask {
        inputs.inline_edit = Some(PixelRect {
            x: x + 16,
            y: cy,
            width: (w - 32).max(60) as u32,
            height: 24,
        });
        cy += 30;
    }

    cy += 6;
    fill_rect(frame, x + 14, cy, w - 28, 1, pal.divider);
    cy += 10;
    push_txt(
        texts,
        (x + 16, cy, 120, 18),
        "Заметки:",
        (12, true),
        pal.text_primary,
        TextAlign::Left,
    );
    cy += 22;

    let body_h = (y + h - cy - 16).max(48);
    inputs.body = Some(PixelRect {
        x: x + 14,
        y: cy,
        width: (w - 28).max(80) as u32,
        height: body_h as u32,
    });
}
