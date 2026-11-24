use asrbox_platform_api::{PixelRect, WidgetLayer};

use crate::layout::{ButtonTone, HitKind, LayoutBuilder};
use crate::layout_tabs::row;
use crate::types::{DockSettingsView, SettingsAction, SettingsSnapshot, SliderKind};

pub(crate) fn build_dock_tab(
    b: &mut LayoutBuilder,
    snap: &SettingsSnapshot,
    selected_idx: usize,
    selected_item: Option<u64>,
) {
    b.set_header(
        "Док-панели",
        "Каждая док-панель настраивается индивидуально; значки можно перетаскивать между панелями",
    );
    let sel_y = b.cursor_y;
    let mut px = b.content_x;
    for idx in 0..snap.docks.len() {
        let r = PixelRect {
            x: px,
            y: b.sy(sel_y),
            width: 92,
            height: 30,
        };
        b.push_control(r, HitKind::SelectDock(idx));
        px += 100;
    }
    b.push_btn(
        (px, sel_y, 112, 30),
        "+ Новый док",
        ButtonTone::Primary,
        SettingsAction::AddDock,
    );
    b.cursor_y = sel_y + 44;
    let Some(dock) = snap.docks.get(selected_idx).or_else(|| snap.docks.first()) else {
        return;
    };
    add_dock_appearance_card(b, dock);
    add_dock_rename_card(b, dock, selected_item);
    add_dock_behavior_card(b, dock, snap.docks.len() > 1);
}

fn add_dock_appearance_card(b: &mut LayoutBuilder, d: &DockSettingsView) {
    let rows = vec![
        row(0, 50, "Край экрана", None, None, true),
        row(50, 50, "Слой отображения", None, None, true),
        row(
            100,
            56,
            "Отображение имён",
            Some("Индивидуально для этой док-панели"),
            None,
            true,
        ),
        row(
            156,
            54,
            "Размер иконок",
            Some("Произвольный размер 24–128 px"),
            Some(format!("{} px", d.icon_size_px)),
            true,
        ),
        row(
            210,
            54,
            "Прозрачность панели",
            Some("Видимость фона панели"),
            Some(format!("{}%", d.opacity_percent)),
            false,
        ),
    ];
    let cy = b.add_card(Some("ВНЕШНИЙ ВИД"), false, rows);
    let mk = |edge_index| SettingsAction::SetDockEdge {
        dock_id: d.id.clone(),
        edge_index,
    };
    b.add_segmented_row(
        cy,
        50,
        vec![
            ("Низ", d.edge_index == 0, mk(0)),
            ("Верх", d.edge_index == 1, mk(1)),
            ("Лево", d.edge_index == 2, mk(2)),
            ("Право", d.edge_index == 3, mk(3)),
        ],
    );
    let mkl = |layer| SettingsAction::SetDockLayer {
        dock_id: d.id.clone(),
        layer,
    };
    b.add_segmented_row(
        cy + 50,
        50,
        vec![
            (
                "Обои",
                d.layer == WidgetLayer::Wallpaper,
                mkl(WidgetLayer::Wallpaper),
            ),
            (
                "Обычный",
                d.layer == WidgetLayer::Normal,
                mkl(WidgetLayer::Normal),
            ),
            (
                "Поверх",
                d.layer == WidgetLayer::Topmost,
                mkl(WidgetLayer::Topmost),
            ),
        ],
    );
    let mkm = |mode_index| SettingsAction::SetDockLabelMode {
        dock_id: d.id.clone(),
        mode_index,
    };
    b.add_segmented_row(
        cy + 100,
        56,
        vec![
            ("Иконки", d.label_mode_index == 0, mkm(0)),
            ("Наведение", d.label_mode_index == 1, mkm(1)),
            ("Иконка+имя", d.label_mode_index == 2, mkm(2)),
            ("Имена", d.label_mode_index == 3, mkm(3)),
        ],
    );
    let sx = b.content_x + b.content_w as i32 - 220;
    b.push_slider(sx, cy + 171, SliderKind::DockIconSize, d.icon_size_px);
    b.push_slider(
        sx,
        cy + 225,
        SliderKind::DockOpacity,
        u32::from(d.opacity_percent),
    );
}

fn add_dock_rename_card(b: &mut LayoutBuilder, d: &DockSettingsView, selected_item: Option<u64>) {
    if d.items.is_empty() {
        let rows = vec![row(
            0,
            52,
            "Нет ярлыков на этой панели",
            Some("Перетащите ярлыки или .exe на док-панель — после этого их можно переименовать"),
            None,
            false,
        )];
        b.add_card(Some("ЯРЛЫКИ И ПЕРЕИМЕНОВАНИЕ"), false, rows);
        return;
    }

    let cols = 3usize;
    let chip_rows = d.items.len().div_ceil(cols) as u32;
    let chips_h = 12 + chip_rows * 34 + 6;
    let active_id = selected_item
        .filter(|id| d.items.iter().any(|i| i.id == *id))
        .unwrap_or(d.items[0].id);

    let rows = vec![
        row(
            0,
            52,
            "Выберите значок этой панели",
            Some("Имя отображается в подсказке и на панели в режимах с подписями"),
            Some(format!("{} шт.", d.items.len())),
            true,
        ),
        row(52, chips_h, "", None, None, true),
        row(52 + chips_h as i32, 54, "", None, None, false),
    ];
    let cy = b.add_card(Some("ЯРЛЫКИ И ПЕРЕИМЕНОВАНИЕ"), false, rows);

    let inner_w = b.content_w.saturating_sub(32);
    let gap = 8u32;
    let chip_w = (inner_w.saturating_sub(gap * (cols as u32 - 1))) / cols as u32;
    for (idx, item) in d.items.iter().enumerate() {
        let col = (idx % cols) as u32;
        let r_idx = (idx / cols) as u32;
        let x = b.content_x + 16 + (col * (chip_w + gap)) as i32;
        let y_log = cy + 62 + (r_idx * 34) as i32;
        b.push_control(
            PixelRect {
                x,
                y: b.sy(y_log),
                width: chip_w,
                height: 28,
            },
            HitKind::SelectDockItem {
                item_id: item.id,
                label: item.label.clone(),
                active: item.id == active_id,
            },
        );
    }

    let edit_y = cy + 52 + chips_h as i32 + 12;
    let save_w = 132u32;
    let reset_w = 96u32;
    let input_w = b
        .content_w
        .saturating_sub(32 + save_w + reset_w + 16)
        .max(140);
    let input_x = b.content_x + 16;
    b.out.input_rects.primary = Some(PixelRect {
        x: input_x,
        y: b.sy(edit_y),
        width: input_w,
        height: 30,
    });
    b.push_btn(
        (input_x + input_w as i32 + 8, edit_y, save_w, 30),
        "Сохранить имя",
        ButtonTone::Primary,
        SettingsAction::RenameDockItemFromInput {
            dock_id: d.id.clone(),
            item_id: active_id,
        },
    );
    b.push_btn(
        (
            input_x + input_w as i32 + 8 + save_w as i32 + 8,
            edit_y,
            reset_w,
            30,
        ),
        "Сбросить",
        ButtonTone::Secondary,
        SettingsAction::ResetDockItemLabel {
            dock_id: d.id.clone(),
            item_id: active_id,
        },
    );
}

fn add_dock_behavior_card(b: &mut LayoutBuilder, d: &DockSettingsView, can_delete: bool) {
    let rows = vec![
        row(
            0,
            52,
            "Закрепить панель",
            Some("Отключите, чтобы перетащить панель за ручку"),
            None,
            true,
        ),
        row(
            52,
            52,
            "Индикатор запущенных программ",
            Some("Точка под открытыми приложениями"),
            None,
            true,
        ),
        row(
            104,
            52,
            "Не запускать вторую копию",
            Some("Если программа уже запущена — открывать её текущее окно"),
            None,
            true,
        ),
        row(
            156,
            52,
            "Анимация запуска",
            Some("Лёгкое подпрыгивание значка при запуске программы"),
            None,
            false,
        ),
    ];
    let cy = b.add_card(Some("ПОВЕДЕНИЕ"), false, rows);
    let tx = b.content_x + b.content_w as i32 - 60;
    let id = d.id.clone();
    b.push_toggle(
        tx,
        cy + 14,
        d.locked,
        SettingsAction::SetDockLocked {
            dock_id: id.clone(),
            locked: !d.locked,
        },
    );
    b.push_toggle(
        tx,
        cy + 66,
        d.show_running_indicator,
        SettingsAction::SetDockShowRunning {
            dock_id: id.clone(),
            enabled: !d.show_running_indicator,
        },
    );
    b.push_toggle(
        tx,
        cy + 118,
        d.focus_running_window,
        SettingsAction::SetDockFocusRunning {
            dock_id: id.clone(),
            enabled: !d.focus_running_window,
        },
    );
    b.push_toggle(
        tx,
        cy + 170,
        d.launch_animation_enabled,
        SettingsAction::SetDockLaunchAnimation {
            dock_id: id.clone(),
            enabled: !d.launch_animation_enabled,
        },
    );
    let y = b.cursor_y;
    let x = b.content_x;
    b.push_btn(
        (x, y, 156, 32),
        "+ Разделитель",
        ButtonTone::Secondary,
        SettingsAction::AddDockSeparator(id.clone()),
    );
    b.push_btn(
        (x + 166, y, 156, 32),
        "Открыть docks.cfg",
        ButtonTone::Secondary,
        SettingsAction::OpenDocksConfig,
    );
    if can_delete {
        b.push_btn(
            (x + 332, y, 118, 32),
            "Удалить док",
            ButtonTone::Danger,
            SettingsAction::RemoveDock(id),
        );
    }
    b.cursor_y += 44;
}
