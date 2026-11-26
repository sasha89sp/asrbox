use std::collections::HashSet;
use std::rc::Rc;

use asrbox_core::{load_modules_config, IsolationMode, ModuleId};
use asrbox_mod_about::AppPaths;
use asrbox_mod_backup::{load_backup_config, load_github_config};
use asrbox_mod_webdav::{load_sync_config, load_webdav_store};
use asrbox_platform_api::{load_appearance_config, AppLanguage, AppearanceConfig, FontRenderer};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::generated::ui::{
    SettingsWindow, SlintModuleRow, SlintSyncItemRow, SlintWebDavServerRow, Theme,
};
use crate::ipc_proto::UiRuntimeStatus;
use crate::settings_state_tabs::{apply_dock_state, apply_overlay_state, apply_widgets_state};

pub struct SettingsViewContext<'a> {
    pub paths: &'a AppPaths,
    pub selected_dock: usize,
    pub selected_dock_item: Option<u64>,
    pub expanded: &'a HashSet<String>,
    pub search: &'a str,
    pub runtime: &'a UiRuntimeStatus,
    pub refresh_inputs: bool,
}

pub fn apply_theme_to_window<C: ComponentHandle>(ui: &C, app: AppearanceConfig) -> bool {
    let dark = app.theme.is_dark();
    let theme = ui.global::<Theme>();
    theme.set_dark_mode(dark);
    theme.set_accent_index(i32::from(app.accent.index()));
    theme.set_glass_mode(app.design_theme.is_glass());
    theme.set_is_ru(app.language == AppLanguage::Ru);
    dark
}

pub fn apply_settings_state(ui: &SettingsWindow, ctx: &SettingsViewContext<'_>) -> (bool, usize) {
    let app = load_appearance_config(&ctx.paths.appearance_cfg);
    let dark = apply_theme_to_window(ui, app);
    ui.set_design_theme_index(i32::from(app.design_theme.index()));
    ui.set_language_index(i32::from(app.language.index()));
    ui.set_font_renderer_index(match app.font_renderer {
        FontRenderer::GdiClearType => 0,
        FontRenderer::DirectWrite => 1,
    });
    ui.set_status_banner(SharedString::from(ctx.runtime.status_banner.as_str()));

    apply_search_and_accordions(ui, ctx.expanded, ctx.search);
    apply_general_state(ui, ctx, app.language);
    let dock_count = apply_dock_state(ui, ctx, app.language);
    apply_widgets_state(ui, ctx);
    apply_overlay_state(ui, ctx);
    (dark, dock_count)
}

fn apply_search_and_accordions(ui: &SettingsWindow, exp: &HashSet<String>, search: &str) {
    let q = search.trim().to_lowercase();
    let has_q = !q.is_empty();
    let hit = |keys: &str| has_q && keys.contains(q.as_str());

    let h_app = hit(
        "оформление тема темная светлая акцент цвет язык шрифт theme accent dark light",
    );
    let h_sys = hit("система автозагрузка изоляция процессы модули сброс autostart isolation");
    let h_bkp = hit("бэкап шифрование aes zip пароль webdav облако github исходный код backup");
    let h_syn = hit("синхронизация файлы папки интервал sync folder webdav");
    let h_tls = hit("диагностика логи отчёт версия о программе data путь debug logs about");
    let h_da = hit("док край слой иконки прозрачность отступ скругление размытие dock blur");
    let h_db = hit("поведение закрепить значки индикатор анимация lock items running");
    let h_di = hit("ярлыки папки разделитель переименовать shortcuts items separator");
    let h_dm = hit("пкм контекстное меню дока context menu");
    let h_om = hit("оверлей игра fso topmost настройка угол overlay game setup");
    let h_ol = hit("размер ширина высота частота пресет width height refresh overlay");

    ui.set_show_appearance(!has_q || h_app);
    ui.set_show_system(!has_q || h_sys);
    ui.set_show_backup(!has_q || h_bkp);
    ui.set_show_sync(!has_q || h_syn);
    ui.set_show_tools(!has_q || h_tls);

    ui.set_hi_appearance(h_app);
    ui.set_hi_system(h_sys);
    ui.set_hi_backup(h_bkp);
    ui.set_hi_sync(h_syn);
    ui.set_hi_tools(h_tls);
    ui.set_hi_dock_appearance(h_da);
    ui.set_hi_dock_behavior(h_db);
    ui.set_hi_dock_items(h_di);
    ui.set_hi_dock_menu(h_dm);
    ui.set_hi_overlay_mode(h_om);
    ui.set_hi_overlay_layout(h_ol);

    ui.set_exp_appearance(h_app || exp.contains("appearance"));
    ui.set_exp_system(h_sys || exp.contains("system"));
    ui.set_exp_backup(h_bkp || exp.contains("backup"));
    ui.set_exp_sync(h_syn || exp.contains("sync"));
    ui.set_exp_tools(h_tls || exp.contains("tools"));
    ui.set_exp_dock_appearance(h_da || exp.contains("dock_appearance"));
    ui.set_exp_dock_behavior(h_db || exp.contains("dock_behavior"));
    ui.set_exp_dock_items(h_di || exp.contains("dock_items"));
    ui.set_exp_dock_menu(h_dm || exp.contains("dock_menu"));
    ui.set_exp_cal(exp.contains("cal_main"));
    ui.set_exp_sys(exp.contains("sys_main"));
    ui.set_exp_weather(exp.contains("weather_main"));
    ui.set_exp_notes(exp.contains("notes_main"));
    ui.set_exp_overlay_mode(h_om || exp.contains("overlay_mode"));
    ui.set_exp_overlay_layout(h_ol || exp.contains("overlay_layout"));
}

fn apply_general_state(ui: &SettingsWindow, ctx: &SettingsViewContext<'_>, lang: AppLanguage) {
    let mod_cfg =
        load_modules_config(&ctx.paths.data_dir.join("config").join("modules.cfg"))
            .unwrap_or_default();
    ui.set_autostart_enabled(ctx.runtime.autostart);
    ui.set_isolation_index(match mod_cfg.isolation {
        IsolationMode::Process => 0,
        IsolationMode::InProc => 1,
    });
    let mod_rows: Vec<SlintModuleRow> = ModuleId::ALL
        .into_iter()
        .map(|id| {
            let live = ctx.runtime.modules.iter().find(|m| m.id == id.as_str());
            let enabled = live.map_or_else(|| mod_cfg.is_enabled(id), |m| m.enabled);
            let running = live.map_or(enabled, |m| m.running);
            let status_text = live
                .map(|m| m.status_text.clone())
                .unwrap_or_else(|| if enabled { "активен" } else { "выключен" }.to_owned());
            SlintModuleRow {
                id: SharedString::from(id.as_str()),
                label: SharedString::from(module_label(id, lang)),
                enabled,
                running,
                status_text: SharedString::from(status_text),
            }
        })
        .collect();
    ui.set_modules(ModelRc::from(Rc::new(VecModel::from(mod_rows))));
    apply_backup_state(ui, ctx, lang);
    apply_sync_state(ui, ctx, lang);
    ui.set_diag_summary(SharedString::from(ctx.runtime.diag_summary.as_str()));
    ui.set_diag_verbose(ctx.runtime.diag_verbose);
    ui.set_diag_file_log(ctx.runtime.diag_file_log);
    ui.set_data_dir_text(SharedString::from(ctx.paths.data_dir.display().to_string()));
}

fn apply_backup_state(ui: &SettingsWindow, ctx: &SettingsViewContext<'_>, lang: AppLanguage) {
    let bkp = load_backup_config(&ctx.paths.backup_cfg);
    let store = load_webdav_store(&ctx.paths.webdav_cfg);
    let sync = load_sync_config(&ctx.paths.sync_cfg);
    let gh = load_github_config(&ctx.paths.github_backup_cfg);

    let bkp_summary = if bkp.last_backup_file.is_empty() {
        if lang == AppLanguage::Ru {
            "Бэкапы ещё не создавались".to_owned()
        } else {
            "No backups yet".to_owned()
        }
    } else if bkp.last_backup_time.is_empty() {
        bkp.last_backup_file.clone()
    } else {
        format!("{} · {}", bkp.last_backup_file, bkp.last_backup_time)
    };
    ui.set_backup_summary(SharedString::from(bkp_summary));
    ui.set_cloud_status(SharedString::from(bkp.last_cloud_status.as_str()));
    ui.set_backup_auto_upload(bkp.auto_upload_cloud);
    ui.set_backup_encrypt_zip(bkp.encrypt_zip);
    ui.set_backup_has_zip_pw(ctx.runtime.has_zip_pw);
    ui.set_backup_busy(ctx.runtime.backup_busy);

    let srv_rows: Vec<SlintWebDavServerRow> = store
        .servers
        .iter()
        .map(|s| SlintWebDavServerRow {
            id: SharedString::from(s.id.as_str()),
            name: SharedString::from(s.display_title()),
            provider_label: SharedString::from(if lang == AppLanguage::Ru {
                s.provider.label_ru()
            } else {
                s.provider.label_en()
            }),
            remote_dir: SharedString::from(s.normalized_remote_dir()),
            status_text: SharedString::new(),
            has_password: true,
            is_backup_target: bkp.target_server_ids.contains(&s.id),
            is_sync_target: sync.target_server_ids.contains(&s.id),
        })
        .collect();
    ui.set_webdav_servers(ModelRc::from(Rc::new(VecModel::from(srv_rows))));

    if ctx.refresh_inputs {
        ui.set_gh_repo(SharedString::from(gh.repo.as_str()));
        ui.set_gh_branch(SharedString::from(gh.branch.as_str()));
    }
    ui.set_gh_include_in_backup(gh.include_in_backup);
    ui.set_gh_has_token(ctx.runtime.has_gh_tok);
    ui.set_gh_status(SharedString::from(gh.last_status.as_str()));
}

fn apply_sync_state(ui: &SettingsWindow, ctx: &SettingsViewContext<'_>, lang: AppLanguage) {
    let sync = load_sync_config(&ctx.paths.sync_cfg);
    ui.set_sync_interval_index(i32::from(sync.interval.index()));
    let sync_st = if sync.last_sync_status.is_empty() {
        sync.interval.label(lang).to_owned()
    } else {
        sync.last_sync_status.clone()
    };
    ui.set_sync_status(SharedString::from(sync_st));
    let sync_rows: Vec<SlintSyncItemRow> = sync
        .items
        .iter()
        .map(|it| SlintSyncItemRow {
            id: SharedString::from(it.path.as_str()),
            local_path: SharedString::from(it.path.as_str()),
            kind_label: SharedString::from(match (it.is_dir, lang) {
                (true, AppLanguage::Ru) => "Папка",
                (true, AppLanguage::En) => "Folder",
                (false, AppLanguage::Ru) => "Файл",
                (false, AppLanguage::En) => "File",
            }),
        })
        .collect();
    ui.set_sync_items(ModelRc::from(Rc::new(VecModel::from(sync_rows))));
}

fn module_label(id: ModuleId, lang: AppLanguage) -> &'static str {
    match (id, lang) {
        (ModuleId::Dock, AppLanguage::Ru) => "Док-панели",
        (ModuleId::Dock, AppLanguage::En) => "Dock Panels",
        (ModuleId::Widgets, AppLanguage::Ru) => "Виджеты (Календарь / Монитор / Погода)",
        (ModuleId::Widgets, AppLanguage::En) => "Widgets (Calendar / SysMon / Weather)",
        (ModuleId::Overlay, AppLanguage::Ru) => "Игровой оверлей",
        (ModuleId::Overlay, AppLanguage::En) => "Game Overlay",
    }
}
