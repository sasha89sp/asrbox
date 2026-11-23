use std::path::Path;

use crate::{PixelRect, WidgetFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProcessMetrics {
    pub ram_kb: u64,
    pub cpu_tenths_pct: u32,
    pub gdi_handles: u32,
    pub user_handles: u32,
    pub uptime_sec: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugUiCommand {
    ToggleVerbose,
    ToggleFileLog,
    CycleFilter,
    CopyReport,
    SaveReport,
    ClearLogs,
    WindowClosed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AboutUiCommand {
    OpenDataFolder,
    CopyPaths,
    WindowClosed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsUiEvent {
    PointerMove { x: i32, y: i32 },
    MouseDown { x: i32, y: i32 },
    MouseUp { x: i32, y: i32 },
    Wheel { delta: i16 },
    Resized { width: u32, height: u32 },
    Closed,
}

pub trait ToolsPlatform {
    fn open_debug_window(&mut self, title: &str) -> Result<(), String>;

    fn update_debug_window(
        &mut self,
        summary: &str,
        logs: &str,
        verbose: bool,
        file_log: bool,
        filter_label: &str,
    );

    fn is_debug_window_open(&self) -> bool;

    fn open_about_window(&mut self, title: &str, content: &str) -> Result<(), String>;

    fn open_settings_window(&mut self, title: &str, width: u32, height: u32) -> Result<(), String>;

    fn present_settings_frame(&mut self, frame: &WidgetFrame, url_box_rect: Option<PixelRect>);

    fn settings_url_input_text(&self) -> String;

    fn set_settings_url_input_text(&self, text: &str);

    fn is_settings_window_open(&self) -> bool;

    fn query_process_metrics(&mut self) -> ProcessMetrics;

    fn copy_to_clipboard(&self, text: &str) -> Result<(), String>;

    fn open_folder(&self, path: &Path) -> Result<(), String>;

    fn open_text_file(&self, path: &Path) -> Result<(), String>;
}
