use asrbox_platform_api::{PixelPoint, PixelRect};

use crate::layout::{ButtonTone, HitKind, LayoutBuilder};
use crate::layout_tabs::row;
use crate::types::{DockSettingsView, SettingsAction};

const ITEM_ROW_H: i32 = 46;
const ITEM_ROW_GAP: i32 = 6;

pub(crate) fn add_dock_items_card(
    b: &mut LayoutBuilder,
    d: &DockSettingsView,
    selected_item: Option<u64>,
) {
    let summary = format!("{} шт.", d.items.len());
    if !b.accordion(
        "dock_rename",
        "Папки и ярлыки на панели",
        &summary,
        "ярлыки папки переименовать порядок удалить добавить разделитель rename shortcut folder items",
    ) {
        return;
    }

    let active_shortcut = selected_item
        .and_then(|id| d.items.iter().find(|i| i.id == id && !i.is_separator))
        .or_else(|| d.items.iter().find(|i| !i.is_separator));

    let list_h: u32 = if d.items.is_empty() {
        56
    } else {
        12 + (d.items.len() as u32) * (ITEM_ROW_H + ITEM_ROW_GAP) as u32 + 6
    };
    let edit_h: u32 = if active_shortcut.is_some() { 86 } else { 0 };
    let footer_h: u32 = 48;

    let mut rows = Vec::new();
    if d.items.is_empty() {
        rows.push(row(
            0,
            list_h,
            "Нет папок и ярлыков на этой панели",
            Some("Перетащите файлы на док или нажмите «+ Добавить папку / ярлык» ниже"),
            None,
            true,
        ));
    } else {
        rows.push(row(0, list_h, "", None, None, true));
    }
    if edit_h > 0 {
        rows.push(row(list_h as i32, edit_h, "", None, None, true));
    }
    rows.push(row(
        (list_h + edit_h) as i32,
        footer_h,
        "",
        None,
        None,
        false,
    ));

    let cy = b.add_card(None, false, rows);
    let inner_x = b.content_x + 14;
    let inner_w = b.content_w.saturating_sub(28);
    let active_id = active_shortcut.map(|i| i.id);

    for (idx, item) in d.items.iter().enumerate() {
        let ry = cy + 10 + (idx as i32) * (ITEM_ROW_H + ITEM_ROW_GAP);
        let is_active = active_id == Some(item.id);
        b.push_control(
            PixelRect {
                x: inner_x,
                y: b.sy(ry),
                width: inner_w,
                height: ITEM_ROW_H as u32,
            },
            HitKind::DockItemCard {
                item_id: item.id,
                label: item.label.clone(),
                path: item.target_path.clone(),
                is_folder: item.is_folder,
                is_separator: item.is_separator,
                active: is_active,
            },
        );
        push_row_action_buttons(
            b,
            d,
            item.id,
            item.is_separator,
            is_active,
            inner_x,
            inner_w,
            ry,
        );
    }

    if let Some(sel) = active_shortcut {
        push_inline_rename_bar(b, d, sel.id, inner_x, inner_w, cy + list_h as i32 + 10);
    }

    let foot_y = cy + (list_h + edit_h) as i32 + 9;
    let add_w = 196u32;
    b.push_btn(
        (inner_x, foot_y, add_w, 30),
        "+ Добавить папку / ярлык",
        ButtonTone::Primary,
        SettingsAction::AddDockItemMenu {
            dock_id: d.id.clone(),
            anchor_client: PixelPoint {
                x: inner_x,
                y: b.sy(foot_y) + 32,
            },
        },
    );
    b.push_btn(
        (inner_x + add_w as i32 + 8, foot_y, 132, 30),
        "+ Разделитель",
        ButtonTone::Secondary,
        SettingsAction::AddDockSeparator(d.id.clone()),
    );
    b.push_btn(
        (inner_x + add_w as i32 + 148, foot_y, 146, 30),
        "Открыть docks.cfg",
        ButtonTone::Secondary,
        SettingsAction::OpenDocksConfig,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_row_action_buttons(
    b: &mut LayoutBuilder,
    d: &DockSettingsView,
    item_id: u64,
    is_separator: bool,
    is_active: bool,
    inner_x: i32,
    inner_w: u32,
    ry: i32,
) {
    let by = b.sy(ry + 10);
    let mut rx = inner_x + inner_w as i32 - 34;
    b.push_control(
        PixelRect {
            x: rx,
            y: by,
            width: 26,
            height: 26,
        },
        HitKind::IconBtn {
            glyph: "✕".to_owned(),
            danger_hover: true,
            action: SettingsAction::RemoveDockItem {
                dock_id: d.id.clone(),
                item_id,
            },
        },
    );
    rx -= 28;
    if !is_separator {
        b.push_control(
            PixelRect {
                x: rx,
                y: by,
                width: 26,
                height: 26,
            },
            HitKind::SelectDockItem {
                item_id,
                label: "✎".to_owned(),
                active: is_active,
            },
        );
        rx -= 28;
    }
    b.push_control(
        PixelRect {
            x: rx,
            y: by,
            width: 26,
            height: 26,
        },
        HitKind::IconBtn {
            glyph: "↓".to_owned(),
            danger_hover: false,
            action: SettingsAction::MoveDockItem {
                dock_id: d.id.clone(),
                item_id,
                delta: 1,
            },
        },
    );
    rx -= 28;
    b.push_control(
        PixelRect {
            x: rx,
            y: by,
            width: 26,
            height: 26,
        },
        HitKind::IconBtn {
            glyph: "↑".to_owned(),
            danger_hover: false,
            action: SettingsAction::MoveDockItem {
                dock_id: d.id.clone(),
                item_id,
                delta: -1,
            },
        },
    );
}

fn push_inline_rename_bar(
    b: &mut LayoutBuilder,
    d: &DockSettingsView,
    item_id: u64,
    inner_x: i32,
    inner_w: u32,
    edit_y: i32,
) {
    let (save_w, reset_w) = (126u32, 92u32);
    let input_w = inner_w.saturating_sub(save_w + reset_w + 16).max(140);
    b.out.input_rects.primary = Some(PixelRect {
        x: inner_x,
        y: b.sy(edit_y),
        width: input_w,
        height: 30,
    });
    b.push_btn(
        (inner_x + input_w as i32 + 8, edit_y, save_w, 30),
        "Сохранить имя",
        ButtonTone::Primary,
        SettingsAction::RenameDockItemFromInput {
            dock_id: d.id.clone(),
            item_id,
        },
    );
    b.push_btn(
        (
            inner_x + input_w as i32 + 16 + save_w as i32,
            edit_y,
            reset_w,
            30,
        ),
        "Сбросить",
        ButtonTone::Secondary,
        SettingsAction::ResetDockItemLabel {
            dock_id: d.id.clone(),
            item_id,
        },
    );
    let act_y = edit_y + 38;
    b.push_btn(
        (inner_x, act_y, 174, 28),
        "Открыть папку объекта",
        ButtonTone::Secondary,
        SettingsAction::OpenDockItemFolder {
            dock_id: d.id.clone(),
            item_id,
        },
    );
}
