use asrbox_core::ModuleId;
use asrbox_platform_api::{PixelRect, WidgetLayer};

use crate::layout::{ButtonTone, HitKind, LayoutBuilder};
use crate::layout_dev::add_dock_menu_section;
use crate::layout_tabs::{add_module_process_card, row};
use crate::layout_weather_notes::layer_label;
use crate::types::{ComboKind, DockSettingsView, SettingsAction, SettingsSnapshot, SliderKind};

pub(crate) fn build_dock_tab(
    b: &mut LayoutBuilder,
    snap: &SettingsSnapshot,
    selected_idx: usize,
    selected_item: Option<u64>,
) {
    b.set_header(
        "Док-панели",
        "Индивидуальные панели; при свободном перемещении панель не прилипает к краям автоматически",
    );
    if !b.is_searching() {
        add_module_process_card(b, snap, ModuleId::Dock);
        let sel_y = b.cursor_y;
        let mut px = b.content_x;
        for idx in 0..snap.docks.len() {
            b.push_control(
                PixelRect {
                    x: px,
                    y: b.sy(sel_y),
                    width: 92,
                    height: 30,
                },
                HitKind::SelectDock(idx),
            );
            px += 100;
        }
        b.push_btn(
            (px, sel_y, 112, 30),
            "+ Новый док",
            ButtonTone::Primary,
            SettingsAction::AddDock,
        );
        b.cursor_y = sel_y + 42;
    }
    let Some(dock) = snap.docks.get(selected_idx).or_else(|| snap.docks.first()) else {
        return;
    };
    add_dock_appearance_card(b, dock);
    add_dock_behavior_card(b, dock, snap.docks.len() > 1);
    add_dock_rename_card(b, dock, selected_item);
    add_dock_menu_section(b, snap.menus.dock);
}

fn edge_label(idx: u8) -> &'static str {
    match idx {
        1 => "Верх экрана",
        2 => "Левый край",
        3 => "Правый край",
        _ => "Низ экрана",
    }
}

fn label_mode_text(idx: u8) -> &'static str {
    match idx {
        1 => "При наведении",
        2 => "Иконка + имя",
        3 => "Только имена",
        _ => "Только иконки",
    }
}

fn add_dock_appearance_card(b: &mut LayoutBuilder, d: &DockSettingsView) {
    let summary = format!(
        "{} · {} px · {}%",
        edge_label(d.edge_index),
        d.icon_size_px,
        d.opacity_percent
    );
    if !b.accordion(
        "dock_appearance",
        "Внешний вид док-панели",
        &summary,
        "док край слой подписи иконки размер прозрачность dock edge layer icons opacity",
    ) {
        return;
    }
    let rows = vec![
        row(
            0,
            50,
            "Край экрана",
            Some("Прикрепление к краям экрана выполняется здесь или через ПКМ"),
            None,
            true,
        ),
        row(50, 50, "Слой отображения", None, None, true),
        row(
            100,
            50,
            "Отображение имён",
            Some("Индивидуально для этой док-панели"),
            None,
            true,
        ),
        row(
            150,
            52,
            "Размер иконок",
            Some("Произвольный размер 24–128 px"),
            Some(format!("{} px", d.icon_size_px)),
            true,
        ),
        row(
            202,
            52,
            "Прозрачность панели",
            Some("Видимость фона панели"),
            Some(format!("{}%", d.opacity_percent)),
            false,
        ),
    ];
    let cy = b.add_card(None, false, rows);
    let rx = b.content_x + b.content_w as i32 - 14;
    let cw = 196u32;
    let cx = rx - cw as i32;
    b.push_combo(
        cx,
        cy + 10,
        cw,
        edge_label(d.edge_index),
        None,
        ComboKind::DockEdge(d.id.clone()),
    );
    b.push_combo(
        cx,
        cy + 50 + 10,
        cw,
        layer_label(d.layer),
        None,
        ComboKind::DockLayer(d.id.clone()),
    );
    b.push_combo(
        cx,
        cy + 100 + 10,
        cw,
        label_mode_text(d.label_mode_index),
        None,
        ComboKind::DockLabelMode(d.id.clone()),
    );
    let sx = b.content_x + b.content_w as i32 - 220;
    b.push_slider(sx, cy + 164, SliderKind::DockIconSize, d.icon_size_px);
    b.push_slider(
        sx,
        cy + 216,
        SliderKind::DockOpacity,
        u32::from(d.opacity_percent),
    );
}

fn add_dock_behavior_card(b: &mut LayoutBuilder, d: &DockSettingsView, can_delete: bool) {
    let summary = match (d.locked, d.lock_items) {
        (true, true) => "Панель и значки закреплены",
        (false, true) => "Значки закреплены",
        (true, false) => "Панель закреплена",
        (false, false) => "Свободный режим",
    };
    if !b.accordion(
        "dock_behavior",
        "Поведение и закрепление",
        summary,
        "закрепить панель значки перетаскивание индикатор запущенных анимация разделитель lock items animation",
    ) {
        return;
    }
    let rows = vec![
        row(
            0,
            52,
            "Закрепить значки",
            Some("Запретить случайное перетаскивание, перестановку и удаление иконок"),
            None,
            true,
        ),
        row(
            52,
            52,
            "Закрепить панель",
            Some("Отключите, чтобы свободно перемещать панель без авто-прилипания к краям"),
            None,
            true,
        ),
        row(
            104,
            52,
            "Индикатор запущенных программ",
            Some("Акцентная точка над открытыми приложениями"),
            None,
            true,
        ),
        row(
            156,
            52,
            "Не запускать вторую копию",
            Some("Если программа уже запущена — активировать её окно"),
            None,
            true,
        ),
        row(
            208,
            52,
            "Анимация запуска",
            Some("Лёгкое подпрыгивание значка при клике"),
            None,
            false,
        ),
    ];
    let cy = b.add_card(None, false, rows);
    let tx = b.content_x + b.content_w as i32 - 58;
    let id = d.id.clone();
    b.push_toggle(
        tx,
        cy + 15,
        d.lock_items,
        SettingsAction::SetDockLockItems {
            dock_id: id.clone(),
            locked: !d.lock_items,
        },
    );
    b.push_toggle(
        tx,
        cy + 67,
        d.locked,
        SettingsAction::SetDockLocked {
            dock_id: id.clone(),
            locked: !d.locked,
        },
    );
    b.push_toggle(
        tx,
        cy + 119,
        d.show_running_indicator,
        SettingsAction::SetDockShowRunning {
            dock_id: id.clone(),
            enabled: !d.show_running_indicator,
        },
    );
    b.push_toggle(
        tx,
        cy + 171,
        d.focus_running_window,
        SettingsAction::SetDockFocusRunning {
            dock_id: id.clone(),
            enabled: !d.focus_running_window,
        },
    );
    b.push_toggle(
        tx,
        cy + 223,
        d.launch_animation_enabled,
        SettingsAction::SetDockLaunchAnimation {
            dock_id: id.clone(),
            enabled: !d.launch_animation_enabled,
        },
    );

    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 156, 30),
        "+ Разделитель",
        ButtonTone::Secondary,
        SettingsAction::AddDockSeparator(id.clone()),
    );
    b.push_btn(
        (x + 166, y, 156, 30),
        "Открыть docks.cfg",
        ButtonTone::Secondary,
        SettingsAction::OpenDocksConfig,
    );
    if can_delete {
        b.push_btn(
            (x + 332, y, 118, 30),
            "Удалить док",
            ButtonTone::Danger,
            SettingsAction::RemoveDock(id),
        );
    }
    b.cursor_y = y + 40;
}

fn add_dock_rename_card(b: &mut LayoutBuilder, d: &DockSettingsView, selected_item: Option<u64>) {
    let summary = format!("{} шт.", d.items.len());
    if !b.accordion(
        "dock_rename",
        "Ярлыки и переименование",
        &summary,
        "ярлыки переименовать имя значок иконка rename shortcut label",
    ) {
        return;
    }
    if d.items.is_empty() {
        b.add_card(
            None,
            false,
            vec![row(
                0,
                50,
                "Нет ярлыков на этой панели",
                Some("Перетащите ярлыки или .exe на док-панель"),
                None,
                false,
            )],
        );
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
            50,
            "Выберите значок этой панели",
            Some("Имя отображается в подсказке и в режимах с подписями"),
            Some(summary),
            true,
        ),
        row(50, chips_h, "", None, None, true),
        row(50 + chips_h as i32, 52, "", None, None, false),
    ];
    let cy = b.add_card(None, false, rows);

    let inner_w = b.content_w.saturating_sub(32);
    let gap = 8u32;
    let chip_w = (inner_w.saturating_sub(gap * (cols as u32 - 1))) / cols as u32;
    for (idx, item) in d.items.iter().enumerate() {
        let col = (idx % cols) as u32;
        let r_idx = (idx / cols) as u32;
        b.push_control(
            PixelRect {
                x: b.content_x + 16 + (col * (chip_w + gap)) as i32,
                y: b.sy(cy + 60 + (r_idx * 34) as i32),
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

    let edit_y = cy + 50 + chips_h as i32 + 11;
    let (save_w, reset_w) = (130u32, 96u32);
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
            input_x + input_w as i32 + 16 + save_w as i32,
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
    let _ = WidgetLayer::Normal;
}
