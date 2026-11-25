use asrbox_platform_api::{
    DockMenuVisibility, PixelRect, TrayMenuVisibility, WidgetMenuVisibility,
};

use crate::i18n::tr;
use crate::layout::{ButtonTone, LayoutBuilder};
use crate::layout_tabs::row;
use crate::types::{GitHubBackupView, SettingsAction};

pub(crate) fn add_developer_github_section(b: &mut LayoutBuilder, gh: &GitHubBackupView) {
    let l = b.lang;
    let summary = if gh.repo.trim().is_empty() {
        tr(l, "Не настроен", "Not configured").to_owned()
    } else {
        format!("{} ({})", gh.repo, gh.branch)
    };
    let kw = "разработчик github исходники код токен репозиторий ветка developer source repo branch token";
    if !b.accordion(
        "developer_github",
        tr(
            l,
            "Для разработчика · Бэкап исходного кода GitHub",
            "Developer · GitHub Source Backup",
        ),
        &summary,
        kw,
    ) {
        return;
    }
    let token_sub = if gh.has_token {
        tr(
            l,
            "Токен в Credential Manager (оставьте пустым чтобы не менять)",
            "Token saved in Credential Manager (leave blank to keep)",
        )
    } else {
        tr(
            l,
            "Personal Access Token (только в Windows Credential Manager)",
            "Personal Access Token (Windows Credential Manager only)",
        )
    };
    let status_line = if gh.busy {
        tr(
            l,
            "Скачивание архива с GitHub...",
            "Downloading archive from GitHub...",
        )
    } else if gh.last_status.is_empty() {
        tr(
            l,
            "Без установки Git — загрузка zipball через WinHTTP",
            "No Git required — downloads zipball via WinHTTP",
        )
    } else {
        gh.last_status.as_str()
    };
    let rows = vec![
        row(
            0,
            52,
            tr(l, "Репозиторий (owner/repo)", "Repository (owner/repo)"),
            Some(tr(
                l,
                "Например: sasha89sp/asrbox",
                "Example: sasha89sp/asrbox",
            )),
            None,
            true,
        ),
        row(
            52,
            52,
            tr(l, "Ветка (branch)", "Branch"),
            Some(tr(l, "По умолчанию: main", "Default: main")),
            None,
            true,
        ),
        row(
            104,
            54,
            tr(l, "GitHub Token (PAT)", "GitHub Token (PAT)"),
            Some(token_sub),
            None,
            true,
        ),
        row(
            158,
            52,
            tr(l, "Включать исходники в бэкап", "Include source in backups"),
            Some(tr(
                l,
                "Добавлять zipball репозитория внутрь ZIP при бэкапе",
                "Bundle repository zipball inside backup ZIP archives",
            )),
            None,
            true,
        ),
        row(
            210,
            50,
            tr(l, "Статус GitHub", "GitHub status"),
            Some(status_line),
            None,
            false,
        ),
    ];
    let cy = b.add_card(None, false, rows);
    let rx = b.content_x + b.content_w as i32 - 14;
    let inp_w = 240u32.min(b.content_w.saturating_sub(240)).max(150);
    let inp_x = rx - inp_w as i32;
    b.out.input_rects.gh_repo = Some(PixelRect {
        x: inp_x,
        y: b.sy(cy + 12),
        width: inp_w,
        height: 28,
    });
    b.out.input_rects.gh_branch = Some(PixelRect {
        x: inp_x,
        y: b.sy(cy + 64),
        width: inp_w,
        height: 28,
    });
    b.out.input_rects.gh_token = Some(PixelRect {
        x: inp_x,
        y: b.sy(cy + 116),
        width: inp_w,
        height: 28,
    });
    b.push_toggle(
        b.content_x + b.content_w as i32 - 58,
        cy + 158 + 15,
        gh.include_in_backup,
        SettingsAction::SetGitHubIncludeInBackup(!gh.include_in_backup),
    );
    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 162, 30),
        tr(l, "Сохранить настройки", "Save Settings"),
        ButtonTone::Primary,
        SettingsAction::SaveGitHubSettings,
    );
    b.push_btn(
        (x + 170, y, 174, 30),
        tr(l, "Скачать исходники", "Download Source"),
        ButtonTone::Secondary,
        SettingsAction::DownloadGitHubSourceNow,
    );
    if gh.has_token {
        b.push_btn(
            (x + 352, y, 134, 30),
            tr(l, "Удалить токен", "Clear Token"),
            ButtonTone::Danger,
            SettingsAction::ClearGitHubToken,
        );
    }
    b.cursor_y = y + 40;
}

pub(crate) fn add_tray_menu_section(b: &mut LayoutBuilder, vis: TrayMenuVisibility) {
    let l = b.lang;
    let items = vec![
        (
            tr(l, "Автозагрузка с Windows", "Launch with Windows"),
            vis.show_autostart,
            SettingsAction::SetTrayMenuVis(TrayMenuVisibility {
                show_autostart: !vis.show_autostart,
                ..vis
            }),
        ),
        (
            tr(l, "Док-панели", "Dock panels"),
            vis.show_docks,
            SettingsAction::SetTrayMenuVis(TrayMenuVisibility {
                show_docks: !vis.show_docks,
                ..vis
            }),
        ),
        (
            tr(l, "Виджеты", "Widgets"),
            vis.show_widgets,
            SettingsAction::SetTrayMenuVis(TrayMenuVisibility {
                show_widgets: !vis.show_widgets,
                ..vis
            }),
        ),
        (
            tr(l, "Игровой оверлей", "Game overlay"),
            vis.show_overlay,
            SettingsAction::SetTrayMenuVis(TrayMenuVisibility {
                show_overlay: !vis.show_overlay,
                ..vis
            }),
        ),
        (
            tr(l, "Бэкап настроек", "Settings backup"),
            vis.show_backup,
            SettingsAction::SetTrayMenuVis(TrayMenuVisibility {
                show_backup: !vis.show_backup,
                ..vis
            }),
        ),
        (
            tr(l, "Отладка и диагностика", "Debug & Diagnostics"),
            vis.show_debug,
            SettingsAction::SetTrayMenuVis(TrayMenuVisibility {
                show_debug: !vis.show_debug,
                ..vis
            }),
        ),
        (
            tr(l, "О программе", "About"),
            vis.show_about,
            SettingsAction::SetTrayMenuVis(TrayMenuVisibility {
                show_about: !vis.show_about,
                ..vis
            }),
        ),
    ];
    add_menu_toggle_accordion(
        b,
        "tray_menu_vis",
        tr(l, "Контекстное меню трея", "Tray Context Menu"),
        "меню трей правая кнопка пкм пункты скрыть tray context menu right click",
        items,
    );
}

pub(crate) fn add_dock_menu_section(b: &mut LayoutBuilder, vis: DockMenuVisibility) {
    let l = b.lang;
    let items = vec![
        (
            tr(
                l,
                "Действия со значком (папка / удалить)",
                "Icon actions (open folder / remove)",
            ),
            vis.show_item_actions,
            SettingsAction::SetDockMenuVis(DockMenuVisibility {
                show_item_actions: !vis.show_item_actions,
                ..vis
            }),
        ),
        (
            tr(l, "Добавить разделитель", "Add separator"),
            vis.show_add_separator,
            SettingsAction::SetDockMenuVis(DockMenuVisibility {
                show_add_separator: !vis.show_add_separator,
                ..vis
            }),
        ),
        (
            tr(l, "Край экрана и слой окна", "Screen edge & window layer"),
            vis.show_edge_and_layer,
            SettingsAction::SetDockMenuVis(DockMenuVisibility {
                show_edge_and_layer: !vis.show_edge_and_layer,
                ..vis
            }),
        ),
        (
            tr(
                l,
                "Внешний вид (размер / прозрачность)",
                "Appearance (size / opacity)",
            ),
            vis.show_appearance,
            SettingsAction::SetDockMenuVis(DockMenuVisibility {
                show_appearance: !vis.show_appearance,
                ..vis
            }),
        ),
        (
            tr(
                l,
                "Поведение (скрытие / анимация)",
                "Behavior (autohide / animation)",
            ),
            vis.show_behavior,
            SettingsAction::SetDockMenuVis(DockMenuVisibility {
                show_behavior: !vis.show_behavior,
                ..vis
            }),
        ),
        (
            tr(
                l,
                "Закрепление (панель и значки)",
                "Lock controls (panel & icons)",
            ),
            vis.show_lock_controls,
            SettingsAction::SetDockMenuVis(DockMenuVisibility {
                show_lock_controls: !vis.show_lock_controls,
                ..vis
            }),
        ),
        (
            tr(
                l,
                "Управление панелями (+ док / удалить)",
                "Panel management (+ dock / delete)",
            ),
            vis.show_dock_management,
            SettingsAction::SetDockMenuVis(DockMenuVisibility {
                show_dock_management: !vis.show_dock_management,
                ..vis
            }),
        ),
    ];
    add_menu_toggle_accordion(
        b,
        "dock_menu_vis",
        tr(l, "Контекстное меню док-панели", "Dock Context Menu"),
        "меню док правая кнопка пкм пункты dock context menu right click",
        items,
    );
}

pub(crate) fn add_widget_menu_section(b: &mut LayoutBuilder, vis: WidgetMenuVisibility) {
    let l = b.lang;
    let items = vec![
        (
            tr(l, "Обновить сейчас", "Refresh now"),
            vis.show_refresh,
            SettingsAction::SetWidgetMenuVis(WidgetMenuVisibility {
                show_refresh: !vis.show_refresh,
                ..vis
            }),
        ),
        (
            tr(
                l,
                "Слой окна (Обои / Обычный / Поверх)",
                "Window layer (Desktop / Normal / Topmost)",
            ),
            vis.show_layer,
            SettingsAction::SetWidgetMenuVis(WidgetMenuVisibility {
                show_layer: !vis.show_layer,
                ..vis
            }),
        ),
        (
            tr(l, "Прозрачность фона", "Background opacity"),
            vis.show_opacity,
            SettingsAction::SetWidgetMenuVis(WidgetMenuVisibility {
                show_opacity: !vis.show_opacity,
                ..vis
            }),
        ),
        (
            tr(l, "Частота обновления", "Refresh interval"),
            vis.show_interval,
            SettingsAction::SetWidgetMenuVis(WidgetMenuVisibility {
                show_interval: !vis.show_interval,
                ..vis
            }),
        ),
        (
            tr(l, "Открыть конфиг (.cfg)", "Open config (.cfg)"),
            vis.show_open_config,
            SettingsAction::SetWidgetMenuVis(WidgetMenuVisibility {
                show_open_config: !vis.show_open_config,
                ..vis
            }),
        ),
        (
            tr(l, "Скрыть виджет", "Hide widget"),
            vis.show_hide,
            SettingsAction::SetWidgetMenuVis(WidgetMenuVisibility {
                show_hide: !vis.show_hide,
                ..vis
            }),
        ),
    ];
    add_menu_toggle_accordion(
        b,
        "widget_menu_vis",
        tr(l, "Контекстное меню виджетов", "Widgets Context Menu"),
        "меню виджет правая кнопка пкм пункты widget context menu right click",
        items,
    );
}

fn add_menu_toggle_accordion(
    b: &mut LayoutBuilder,
    id: &'static str,
    title: &str,
    keywords: &str,
    items: Vec<(&str, bool, SettingsAction)>,
) {
    let total = items.len();
    let enabled_cnt = items.iter().filter(|(_, on, _)| *on).count();
    let summary = format!("{enabled_cnt}/{total} {}", tr(b.lang, "пунктов", "items"));
    if !b.accordion(id, title, &summary, keywords) {
        return;
    }
    let rows: Vec<_> = items
        .iter()
        .enumerate()
        .map(|(i, (lbl, _, _))| row((i as i32) * 44, 44, lbl, None, None, i + 1 < total))
        .collect();
    let cy = b.add_card(None, false, rows);
    let tx = b.content_x + b.content_w as i32 - 58;
    for (i, (_, on, act)) in items.into_iter().enumerate() {
        b.push_toggle(tx, cy + (i as i32) * 44 + 11, on, act);
    }
}
