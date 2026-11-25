use asrbox_core::ModuleId;

use crate::layout::{ButtonTone, HitKind, LayoutBuilder, TAB_STRIP_H};
use crate::layout_dev::add_dock_menu_section;
use crate::layout_dock_items::add_dock_items_card;
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
        "Индивидуальные панели; в свободном режиме панель не прилипает к краям автоматически",
    );
    let Some(dock) = snap.docks.get(selected_idx).or_else(|| snap.docks.first()) else {
        return;
    };
    if !b.is_searching() {
        add_dock_header_controls(b, dock);
        add_module_process_card(b, snap, ModuleId::Dock);
    }
    add_dock_appearance_card(b, dock);
    add_dock_behavior_card(b, dock, snap.docks.len() > 1);
    add_dock_items_card(b, dock, selected_item);
    add_dock_menu_section(b, snap.menus.dock);
}

fn add_dock_header_controls(b: &mut LayoutBuilder, dock: &DockSettingsView) {
    let sel_label = format!("Док {} ({})", dock.id, edge_short(dock.edge_index));
    if b.content_w >= 560 {
        b.resize_header_search(168);
        let hy = if b.out.has_tab_strip { TAB_STRIP_H } else { 0 };
        let cy = hy + 13;
        let combo_x = b.content_x + 148;
        let combo_w = 182u32;
        b.push_header_control(
            asrbox_platform_api::PixelRect {
                x: combo_x,
                y: cy,
                width: combo_w,
                height: 30,
            },
            HitKind::ComboBox {
                label: sel_label,
                dot_color: None,
                kind: ComboKind::DockSelect,
            },
        );
        b.push_header_control(
            asrbox_platform_api::PixelRect {
                x: combo_x + combo_w as i32 + 8,
                y: cy,
                width: 112,
                height: 30,
            },
            HitKind::Button {
                label: "+ Новый док".to_owned(),
                tone: ButtonTone::Secondary,
                action: SettingsAction::AddDock,
            },
        );
    } else {
        let sel_y = b.cursor_y;
        b.push_combo(
            b.content_x,
            sel_y,
            210,
            &sel_label,
            None,
            ComboKind::DockSelect,
        );
        b.push_btn(
            (b.content_x + 220, sel_y, 118, 30),
            "+ Новый док",
            ButtonTone::Secondary,
            SettingsAction::AddDock,
        );
        b.cursor_y = sel_y + 42;
    }
}

fn edge_short(idx: u8) -> &'static str {
    match idx {
        1 => "Верх",
        2 => "Лево",
        3 => "Право",
        _ => "Низ",
    }
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
        edge_short(d.edge_index),
        d.icon_size_px,
        d.opacity_percent
    );
    if !b.accordion(
        "dock_appearance",
        "Внешний вид и расположение",
        &summary,
        "док край слой подписи иконки размер прозрачность отступ скругление размытие blur margin radius dock edge",
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
            "Отступ от края экрана",
            Some("Дистанция от границы рабочей области (0–64 px)"),
            Some(format!("{} px", d.edge_margin_px)),
            true,
        ),
        row(
            254,
            52,
            "Скругление углов",
            Some("Радиус капсулы док-панели (0–28 px)"),
            Some(format!("{} px", d.corner_radius_px)),
            true,
        ),
        row(
            306,
            52,
            "Прозрачность панели",
            Some("Непрозрачность стеклянной подложки"),
            Some(format!("{}%", d.opacity_percent)),
            true,
        ),
        row(
            358,
            50,
            "Размытие фона",
            Some("Стеклянный эффект с верхним бликом на капсуле"),
            None,
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
        cy + 60,
        cw,
        layer_label(d.layer),
        None,
        ComboKind::DockLayer(d.id.clone()),
    );
    b.push_combo(
        cx,
        cy + 110,
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
        SliderKind::DockEdgeMargin,
        u32::from(d.edge_margin_px),
    );
    b.push_slider(
        sx,
        cy + 268,
        SliderKind::DockCornerRadius,
        u32::from(d.corner_radius_px),
    );
    b.push_slider(
        sx,
        cy + 320,
        SliderKind::DockOpacity,
        u32::from(d.opacity_percent),
    );
    let tx = b.content_x + b.content_w as i32 - 58;
    b.push_toggle(
        tx,
        cy + 372,
        d.blur_enabled,
        SettingsAction::SetDockBlur {
            dock_id: d.id.clone(),
            enabled: !d.blur_enabled,
        },
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
        "закрепить панель значки перетаскивание индикатор запущенных анимация lock items animation",
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
            Some("Акцентная точка под открытыми приложениями"),
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

    if can_delete {
        let (x, y) = (b.content_x, b.cursor_y);
        b.push_btn(
            (x, y, 136, 30),
            "Удалить этот док",
            ButtonTone::Danger,
            SettingsAction::RemoveDock(id),
        );
        b.cursor_y = y + 40;
    }
}
