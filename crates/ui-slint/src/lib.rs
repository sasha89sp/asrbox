mod dock_icons;
mod dock_runner;
mod dock_view;
mod generated;
mod ipc_proto;
mod notes_bridge;
mod notes_view;
mod settings_bridge;
mod settings_state;
mod settings_state_tabs;
mod tray_runner;
mod vibrancy;
mod widgets_calendar;
mod widgets_menu;
mod widgets_runner;
mod widgets_sysmon;
mod widgets_sysmon_view;
mod widgets_weather;
mod widgets_weather_data;
mod win_os;

pub use dock_runner::run_dock_host;
pub use ipc_proto::{
    decode_runtime_status, decode_tray_show, decode_ui_cmd, encode_runtime_status,
    encode_tray_show, encode_ui_cmd, unescape_field, TrayShowPayload, UiModuleStatus,
    UiRuntimeStatus,
};
pub use notes_bridge::run_notes_window;
pub use settings_bridge::run_settings_window;
pub use tray_runner::run_tray_menu_host;
pub use widgets_runner::run_widgets_host;

pub fn selftest_slint_ipc() -> Result<(), String> {
    use asrbox_platform_api::{TrayMenuEntry, TrayMenuVisibility, TrayModuleFlags};

    let enc = encode_ui_cmd("weather_city", "Москва | Центр");
    let dec = decode_ui_cmd(&enc).ok_or_else(|| "slint UI_CMD decode failed".to_owned())?;
    if dec.0 != "weather_city" || dec.1 != "Москва | Центр" {
        return Err("slint UI_CMD roundtrip mismatch".to_owned());
    }
    let st = UiRuntimeStatus {
        autostart: true,
        has_zip_pw: true,
        has_ya_key: true,
        diag_summary: "RAM 15.0 МБ".to_owned(),
        modules: vec![UiModuleStatus {
            id: "widgets".to_owned(),
            enabled: true,
            running: true,
            status_text: "PID 42".to_owned(),
        }],
        ..UiRuntimeStatus::default()
    };
    if decode_runtime_status(&encode_runtime_status(&st)) != Some(st) {
        return Err("slint UiRuntimeStatus roundtrip mismatch".to_owned());
    }
    let tray = TrayShowPayload {
        x: 1600,
        y: 960,
        flags: TrayModuleFlags {
            dock_module_enabled: true,
            widgets_module_enabled: true,
            overlay_module_enabled: false,
            overlay_setup_mode: false,
            calendar_visible: true,
            sys_monitor_visible: true,
            weather_visible: true,
            notes_visible: false,
            autostart_enabled: false,
            menu_vis: TrayMenuVisibility::default(),
        },
        docks: vec![TrayMenuEntry {
            dock_id: 1,
            title: "Док 1".to_owned(),
            enabled: true,
        }],
    };
    if decode_tray_show(&encode_tray_show(&tray)) != Some(tray) {
        return Err("slint TrayShowPayload roundtrip mismatch".to_owned());
    }
    Ok(())
}
