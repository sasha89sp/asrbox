use asrbox_core::IsolationMode;

use crate::i18n::tr;
use crate::layout::{ButtonTone, CardRow, LayoutBuilder};
use crate::types::{SettingsAction, SettingsSnapshot};

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

pub(crate) fn build_modules_tab(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let lang = b.lang;
    b.set_header(
        tr(
            lang,
            "Модули и изоляция процессов",
            "Modules & Process Isolation",
        ),
        tr(
            lang,
            "Отключённые модули не расходуют CPU/RAM; ошибка в одном модуле не роняет остальные",
            "Disabled modules consume 0 CPU/RAM; isolated workers keep failures contained",
        ),
    );
    if b.matches_search("изоляция процессы один процесс isolation process inproc")
    {
        let iso_sub = Some(tr(
            lang,
            "Отдельный процесс для каждого модуля или общий",
            "Separate worker process per module or shared single process",
        ));
        let iso_y = b.add_card(
            Some(tr(lang, "ИЗОЛЯЦИЯ", "ISOLATION")),
            false,
            vec![row(
                0,
                56,
                tr(lang, "Изоляция модулей", "Module isolation"),
                iso_sub,
                None,
                false,
            )],
        );
        let is_proc = snap.isolation == IsolationMode::Process;
        b.add_segmented_row(
            iso_y,
            56,
            vec![
                (
                    tr(lang, "Процессы", "Processes"),
                    is_proc,
                    SettingsAction::SetIsolation(IsolationMode::Process),
                ),
                (
                    tr(lang, "Один процесс", "In-Process"),
                    !is_proc,
                    SettingsAction::SetIsolation(IsolationMode::InProc),
                ),
            ],
        );
    }

    let visible_mods: Vec<_> = snap
        .modules
        .iter()
        .filter(|m| b.matches_search(m.id.display_name()))
        .collect();
    if visible_mods.is_empty() {
        return;
    }
    let rows: Vec<CardRow> = visible_mods
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let st = match (m.enabled, m.running, m.pid) {
                (false, _, _) => tr(lang, "Отключён (0 МБ RAM)", "Disabled (0 MB RAM)").to_owned(),
                (true, true, Some(p)) => format!(
                    "{} · PID {p} · {}: {}",
                    tr(lang, "Работает", "Running"),
                    tr(lang, "рестартов", "restarts"),
                    m.restarts
                ),
                (true, true, None) => tr(lang, "Работает (inproc)", "Running (inproc)").to_owned(),
                (true, false, _) => {
                    format!("{} ({})", tr(lang, "Остановлен", "Stopped"), m.note)
                }
            };
            row(
                (i as i32) * 60,
                60,
                m.id.display_name(),
                Some(&st),
                None,
                i + 1 < visible_mods.len(),
            )
        })
        .collect();
    let card_y = b.add_card(
        Some(tr(lang, "АКТИВНЫЕ МОДУЛИ", "ACTIVE MODULES")),
        false,
        rows,
    );
    let tx = b.content_x + b.content_w as i32 - 58;
    for (i, m) in visible_mods.iter().enumerate() {
        let ry = card_y + (i as i32) * 60;
        let act = SettingsAction::SetModuleEnabled {
            id: m.id,
            enabled: !m.enabled,
        };
        b.push_toggle(tx, ry + 19, m.enabled, act);
        if m.enabled {
            b.push_btn(
                (tx - 108, ry + 16, 96, 28),
                tr(lang, "Перезапуск", "Restart"),
                ButtonTone::Secondary,
                SettingsAction::RestartModule(m.id),
            );
        }
    }
}
