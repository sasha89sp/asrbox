use asrbox_core::ModuleId;
use asrbox_platform_api::PixelRect;

use crate::i18n::tr;
use crate::layout::{ButtonTone, CardRow, HitKind, LayoutBuilder};
use crate::layout_cal::build_calendar_body;
use crate::layout_dev::add_widget_menu_section;
use crate::layout_sys::build_sysmon_body;
use crate::layout_weather_notes::{build_notes_body, build_weather_body};
use crate::types::{SettingsAction, SettingsSnapshot, WidgetSubTab};

pub(crate) fn row(
    y: i32,
    h: u32,
    title: &str,
    sub: Option<&str>,
    val: Option<String>,
    div: bool,
) -> CardRow {
    CardRow {
        y,
        height: h,
        title: title.to_owned(),
        subtitle: sub.map(str::to_owned),
        value_label: val,
        divider: div,
    }
}

pub(crate) fn add_module_process_card(
    b: &mut LayoutBuilder,
    snap: &SettingsSnapshot,
    id: ModuleId,
) {
    let lang = b.lang;
    let found = snap.modules.iter().find(|m| m.id == id);
    let enabled = found.is_none_or(|m| m.enabled);
    let status = match found {
        None => tr(lang, "Модуль активен", "Module active").to_owned(),
        Some(m) if !m.enabled => tr(lang, "Отключён (0 МБ RAM)", "Disabled (0 MB RAM)").to_owned(),
        Some(m) if m.running => match m.pid {
            Some(pid) => format!(
                "{} · PID {pid} · {}: {}",
                tr(lang, "Работает", "Running"),
                tr(lang, "рестартов", "restarts"),
                m.restarts
            ),
            None => tr(lang, "Работает (в общем процессе)", "Running (in-process)").to_owned(),
        },
        Some(m) => format!("{} ({})", tr(lang, "Остановлен", "Stopped"), m.note),
    };
    let card_y = b.add_card(
        Some(tr(lang, "МОДУЛЬ И ПРОЦЕСС", "MODULE & PROCESS")),
        false,
        vec![row(0, 56, id.display_name(), Some(&status), None, false)],
    );
    let tx = b.content_x + b.content_w as i32 - 58;
    b.push_toggle(
        tx,
        card_y + 17,
        enabled,
        SettingsAction::SetModuleEnabled {
            id,
            enabled: !enabled,
        },
    );
    if enabled {
        b.push_btn(
            (tx - 110, card_y + 14, 98, 28),
            tr(lang, "Перезапуск", "Restart"),
            ButtonTone::Secondary,
            SettingsAction::RestartModule(id),
        );
    }
}

pub(crate) fn build_widgets_tab(
    b: &mut LayoutBuilder,
    snap: &SettingsSnapshot,
    selected: WidgetSubTab,
) {
    let lang = b.lang;
    b.set_header(
        tr(lang, "Виджеты", "Widgets"),
        tr(
            lang,
            "Календарь, Системный монитор, Погода и Заметки работают в одном изолированном процессе",
            "Calendar, System Monitor, Weather and Notes run inside one isolated worker process",
        ),
    );
    add_module_process_card(b, snap, ModuleId::Widgets);

    let sel_y = b.cursor_y;
    let tabs = [
        (WidgetSubTab::Calendar, tr(lang, "Календарь", "Calendar")),
        (WidgetSubTab::SysMonitor, tr(lang, "Монитор", "Monitor")),
        (WidgetSubTab::Weather, tr(lang, "Погода", "Weather")),
        (WidgetSubTab::Notes, tr(lang, "Заметки", "Notes")),
    ];
    let gap = 8i32;
    let tab_w = ((b.content_w as i32 - gap * 3) / 4).max(96) as u32;
    for (idx, (sub, label)) in tabs.into_iter().enumerate() {
        b.push_control(
            PixelRect {
                x: b.content_x + (idx as i32) * (tab_w as i32 + gap),
                y: b.sy(sel_y),
                width: tab_w,
                height: 32,
            },
            HitKind::SelectWidgetSubTab {
                sub,
                label: label.to_owned(),
                active: selected == sub,
            },
        );
    }
    b.cursor_y = sel_y + 44;

    match selected {
        WidgetSubTab::Calendar => build_calendar_body(b, snap),
        WidgetSubTab::SysMonitor => build_sysmon_body(b, snap),
        WidgetSubTab::Weather => build_weather_body(b, snap),
        WidgetSubTab::Notes => build_notes_body(b, snap),
    }
    add_widget_menu_section(b, snap.menus.widget);
}

pub(crate) fn build_all_widgets_for_search(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    if b.matches_search("календарь ics события период google calendar") {
        build_calendar_body(b, snap);
    }
    if b.matches_search("системный монитор cpu gpu ram диск процессы monitor")
    {
        build_sysmon_body(b, snap);
    }
    build_weather_body(b, snap);
    build_notes_body(b, snap);
    add_widget_menu_section(b, snap.menus.widget);
}
