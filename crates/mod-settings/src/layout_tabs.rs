use asrbox_core::ModuleId;
use asrbox_platform_api::PixelRect;

use crate::i18n::tr;
use crate::layout::{ButtonTone, CardRow, HitKind, LayoutBuilder};
use crate::layout_cal::build_calendar_body;
use crate::layout_sys::build_sysmon_body;
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
        vec![row(0, 58, id.display_name(), Some(&status), None, false)],
    );
    let tx = b.content_x + b.content_w as i32 - 58;
    b.push_toggle(
        tx,
        card_y + 18,
        enabled,
        SettingsAction::SetModuleEnabled {
            id,
            enabled: !enabled,
        },
    );
    if enabled {
        b.push_btn(
            (tx - 110, card_y + 15, 98, 28),
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
            "Google Календарь и Системный монитор работают в одном изолированном процессе",
            "Google Calendar and System Monitor run inside one isolated worker process",
        ),
    );
    add_module_process_card(b, snap, ModuleId::Widgets);

    let sel_y = b.cursor_y;
    let x = b.content_x;
    let cal_w = 178u32;
    let sys_w = 186u32;
    b.push_control(
        PixelRect {
            x,
            y: b.sy(sel_y),
            width: cal_w,
            height: 32,
        },
        HitKind::SelectWidgetSubTab {
            sub: WidgetSubTab::Calendar,
            label: tr(lang, "Google Календарь", "Google Calendar").to_owned(),
            active: selected == WidgetSubTab::Calendar,
        },
    );
    b.push_control(
        PixelRect {
            x: x + cal_w as i32 + 8,
            y: b.sy(sel_y),
            width: sys_w,
            height: 32,
        },
        HitKind::SelectWidgetSubTab {
            sub: WidgetSubTab::SysMonitor,
            label: tr(lang, "Системный монитор", "System Monitor").to_owned(),
            active: selected == WidgetSubTab::SysMonitor,
        },
    );
    b.cursor_y = sel_y + 46;

    match selected {
        WidgetSubTab::Calendar => build_calendar_body(b, snap),
        WidgetSubTab::SysMonitor => build_sysmon_body(b, snap),
    }
}
