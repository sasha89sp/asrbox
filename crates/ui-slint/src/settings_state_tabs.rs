use std::rc::Rc;

use asrbox_core::load_modules_config;
use asrbox_mod_dock::{load_docks_from_file, DockConfig, DockEdge};
use asrbox_mod_overlay::load_overlay_config;
use asrbox_mod_widgets::{
    load_notes_config, load_sysmon_config, load_weather_config, load_widget_config, PeriodFilter,
    WeatherProvider,
};
use asrbox_platform_api::{load_menu_config, AppLanguage, WidgetLayer};
use slint::{ModelRc, SharedString, VecModel};

use crate::generated::ui::{SettingsWindow, SlintCalendarSourceRow, SlintDockItemRow};
use crate::settings_state::SettingsViewContext;

pub fn apply_dock_state(
    ui: &SettingsWindow,
    ctx: &SettingsViewContext<'_>,
    lang: AppLanguage,
) -> usize {
    let mut docks = load_docks_from_file(&ctx.paths.docks_cfg).unwrap_or_default();
    if docks.is_empty() {
        docks.push(DockConfig::new(1, "Док 1"));
    }
    let idx = ctx.selected_dock.min(docks.len().saturating_sub(1));
    let names: Vec<SharedString> = docks
        .iter()
        .map(|d| SharedString::from(d.name.as_str()))
        .collect();
    ui.set_dock_names(ModelRc::from(Rc::new(VecModel::from(names))));
    ui.set_selected_dock_index(idx as i32);
    ui.set_can_remove_dock(docks.len() > 1);
    if let Some(d) = docks.get(idx) {
        apply_selected_dock(ui, d, ctx.selected_dock_item, lang);
    }
    let menus = load_menu_config(&ctx.paths.menus_cfg);
    ui.set_dock_menu_open_folder(menus.dock.show_item_actions);
    ui.set_dock_menu_launch(menus.dock.show_add_separator);
    ui.set_dock_menu_remove(menus.dock.show_edge_and_layer);
    ui.set_dock_menu_add_dock(menus.dock.show_dock_management);
    docks.len()
}

fn apply_selected_dock(
    ui: &SettingsWindow,
    d: &DockConfig,
    selected_item: Option<u64>,
    lang: AppLanguage,
) {
    ui.set_selected_dock_name(SharedString::from(d.name.as_str()));
    ui.set_dock_edge_index(match d.edge {
        DockEdge::Bottom => 0,
        DockEdge::Top => 1,
        DockEdge::Left => 2,
        DockEdge::Right => 3,
    });
    ui.set_dock_layer_index(layer_to_idx(d.layer));
    ui.set_dock_label_mode_index(i32::from(d.label_mode.to_index()));
    ui.set_dock_icon_size_px(i32::from(d.icon_size_px));
    ui.set_dock_icon_frame_px(i32::from(d.icon_frame_px.min(24)));
    ui.set_dock_opacity_percent(i32::from(d.opacity_percent));
    ui.set_dock_edge_margin_px(i32::from(d.edge_margin_px));
    ui.set_dock_corner_radius_px(i32::from(d.corner_radius_px));
    ui.set_dock_blur_enabled(d.blur_enabled);
    ui.set_dock_locked(d.locked);
    ui.set_dock_lock_items(d.lock_items);
    ui.set_dock_show_running(d.show_running_indicator);
    ui.set_dock_focus_running(d.focus_running_window);
    ui.set_dock_launch_anim(d.launch_animation_enabled);

    let items: Vec<SlintDockItemRow> = d
        .items
        .iter()
        .map(|it| {
            let is_sep = it.is_separator();
            let is_dir = !is_sep && it.target.is_dir();
            SlintDockItemRow {
                id: it.id as i32,
                label: SharedString::from(if is_sep {
                    "───"
                } else {
                    it.label.as_str()
                }),
                target_path: SharedString::from(it.target.display().to_string()),
                kind_label: SharedString::from(dock_item_kind(is_sep, is_dir, lang)),
                is_folder: is_dir,
                is_separator: is_sep,
                selected: selected_item == Some(it.id),
            }
        })
        .collect();
    ui.set_dock_items(ModelRc::from(Rc::new(VecModel::from(items))));
}

fn dock_item_kind(is_sep: bool, is_dir: bool, lang: AppLanguage) -> &'static str {
    match (is_sep, is_dir, lang) {
        (true, _, AppLanguage::Ru) => "Разделитель",
        (true, _, AppLanguage::En) => "Separator",
        (false, true, AppLanguage::Ru) => "Папка",
        (false, true, AppLanguage::En) => "Folder",
        (false, false, AppLanguage::Ru) => "Программа",
        (false, false, AppLanguage::En) => "App",
    }
}

pub fn apply_widgets_state(ui: &SettingsWindow, ctx: &SettingsViewContext<'_>) {
    apply_calendar_and_sysmon_state(ui, ctx);
    apply_weather_and_notes_state(ui, ctx);
}

fn apply_calendar_and_sysmon_state(ui: &SettingsWindow, ctx: &SettingsViewContext<'_>) {
    let cal = load_widget_config(&ctx.paths.widgets_cfg).unwrap_or_default();
    ui.set_cal_visible(cal.visible);
    ui.set_cal_layer(layer_to_idx(cal.layer));
    ui.set_cal_period(match cal.period {
        PeriodFilter::Today => 0,
        PeriodFilter::Days7 => 1,
        PeriodFilter::Days30 => 2,
        PeriodFilter::All => 3,
    });
    ui.set_cal_sync_idx(match cal.refresh_interval_sec {
        0..=450 => 0,
        451..=1350 => 1,
        1351..=2700 => 2,
        2701..=7200 => 3,
        _ => 4,
    });
    ui.set_cal_opacity(i32::from(cal.opacity_percent));
    ui.set_cal_width(cal.width as i32);
    ui.set_cal_rows(i32::from(cal.visible_rows));
    ui.set_cal_commented_hint(cal.commented_calendar_hint);
    let srcs: Vec<SlintCalendarSourceRow> = cal
        .sources
        .iter()
        .enumerate()
        .map(|(i, s)| SlintCalendarSourceRow {
            index: i as i32,
            id: SharedString::from(s.id.as_str()),
            url: SharedString::from(s.url.as_str()),
        })
        .collect();
    ui.set_cal_sources(ModelRc::from(Rc::new(VecModel::from(srcs))));

    let sys = load_sysmon_config(&ctx.paths.sysmon_cfg).unwrap_or_default();
    ui.set_sys_visible(sys.visible);
    ui.set_sys_layer(layer_to_idx(sys.layer));
    ui.set_sys_refresh_idx(match sys.refresh_interval_ms {
        0..=1499 => 0,
        1500..=2499 => 1,
        2500..=3999 => 2,
        4000..=7499 => 3,
        _ => 4,
    });
    ui.set_sys_opacity(i32::from(sys.opacity_percent));
    ui.set_sys_width(sys.width as i32);
    ui.set_sys_height(sys.height as i32);
}

fn apply_weather_and_notes_state(ui: &SettingsWindow, ctx: &SettingsViewContext<'_>) {
    let w = load_weather_config(&ctx.paths.weather_cfg).unwrap_or_default();
    ui.set_weather_visible(w.visible);
    if ctx.refresh_inputs {
        ui.set_weather_city(SharedString::from(w.city.as_str()));
    }
    ui.set_weather_provider_idx(if w.provider == WeatherProvider::Yandex {
        1
    } else {
        0
    });
    ui.set_weather_days_idx(match w.forecast_days {
        1 => 0,
        7 => 2,
        _ => 1,
    });
    ui.set_weather_has_yandex_key(ctx.runtime.has_ya_key);
    ui.set_weather_layer(layer_to_idx(w.layer));
    ui.set_weather_opacity(i32::from(w.opacity_percent));
    ui.set_weather_refresh_min(w.refresh_interval_min as i32);
    ui.set_weather_width(w.width as i32);
    ui.set_weather_height(w.height as i32);

    let n = load_notes_config(&ctx.paths.notes_cfg).unwrap_or_default();
    ui.set_notes_visible(n.visible);
    ui.set_notes_layer(layer_to_idx(n.layer));
    ui.set_notes_opacity(i32::from(n.opacity_percent));
    ui.set_notes_pane_folders(n.show_folders_pane);
    ui.set_notes_pane_list(n.show_list_pane);
    ui.set_notes_pane_editor(n.show_editor_pane);
}

pub fn apply_overlay_state(ui: &SettingsWindow, ctx: &SettingsViewContext<'_>) {
    let mod_cfg = load_modules_config(&ctx.paths.data_dir.join("config").join("modules.cfg"))
        .unwrap_or_default();
    let ov = load_overlay_config(&ctx.paths.overlay_cfg).unwrap_or_default();
    ui.set_ov_module_enabled(mod_cfg.overlay_enabled);
    ui.set_ov_auto_games_only(ov.auto_games_only);
    ui.set_ov_force_topmost(ov.force_topmost_fullscreen);
    ui.set_ov_setup_mode(ctx.runtime.overlay_setup);
    ui.set_ov_anchor_index(i32::from(ov.anchor.to_index()));
    ui.set_ov_opacity(i32::from(ov.opacity_percent));
    ui.set_ov_width(ov.width as i32);
    ui.set_ov_height(ov.height as i32);
    ui.set_ov_refresh_ms(ov.refresh_interval_ms as i32);
}

fn layer_to_idx(layer: WidgetLayer) -> i32 {
    match layer {
        WidgetLayer::Wallpaper => 0,
        WidgetLayer::Normal => 1,
        WidgetLayer::Topmost => 2,
    }
}
