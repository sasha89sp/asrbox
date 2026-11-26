mod generated;
mod ipc_proto;
mod notes_bridge;
mod notes_view;
mod settings_bridge;
mod settings_state;
mod settings_state_tabs;
mod vibrancy;

pub use ipc_proto::{
    decode_runtime_status, decode_ui_cmd, encode_runtime_status, encode_ui_cmd, unescape_field,
    UiModuleStatus, UiRuntimeStatus,
};
pub use notes_bridge::run_notes_window;
pub use settings_bridge::run_settings_window;

pub fn selftest_slint_ipc() -> Result<(), String> {
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
    Ok(())
}
