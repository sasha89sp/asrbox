use std::path::{Path, PathBuf};

use crate::theme::FluentPalette;
use crate::{PixelPoint, PixelRect, WidgetFrame};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FluentQuickBtn {
    pub id: u32,
    pub glyph: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FluentMenuItem {
    Action {
        id: u32,
        icon: Option<String>,
        label: String,
        shortcut: Option<String>,
        checked: bool,
        enabled: bool,
    },
    Submenu {
        icon: Option<String>,
        label: String,
        enabled: bool,
        items: Vec<FluentMenuItem>,
    },
    Separator,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FluentMenuSpec {
    pub quick_actions: Vec<FluentQuickBtn>,
    pub items: Vec<FluentMenuItem>,
    pub min_width: i32,
}

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
    InputChanged { slot: u8, text: String },
    AnimTick { dt_ms: u16 },
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SettingsInputRects {
    pub primary: Option<PixelRect>,
    pub secret: Option<PixelRect>,
    pub extra: Option<PixelRect>,
    pub search: Option<PixelRect>,
    pub aux: Option<PixelRect>,
    pub zip_secret: Option<PixelRect>,
    pub gh_repo: Option<PixelRect>,
    pub gh_branch: Option<PixelRect>,
    pub gh_token: Option<PixelRect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NotesInputRects {
    pub search: Option<PixelRect>,
    pub title: Option<PixelRect>,
    pub body: Option<PixelRect>,
    pub inline_edit: Option<PixelRect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotesUiEvent {
    PointerMove {
        x: i32,
        y: i32,
    },
    MouseDown {
        x: i32,
        y: i32,
    },
    MouseUp {
        x: i32,
        y: i32,
    },
    RightClick {
        x: i32,
        y: i32,
        screen_x: i32,
        screen_y: i32,
    },
    Wheel {
        x: i32,
        y: i32,
        delta: i16,
    },
    Moved {
        x: i32,
        y: i32,
    },
    Resized {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        finished: bool,
    },
    InputChanged {
        slot: u8,
        text: String,
    },
    InlineSubmit,
    InlineCancel,
    TogglePane(u8),
    FlushSaveTick,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubFetchResult {
    pub op_id: u64,
    pub ok: bool,
    pub status_code: u16,
    pub message: String,
    pub archive_path: Option<PathBuf>,
    pub include_in_pending_backup: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WebDavOpKind {
    #[default]
    Test,
    Upload,
    ListDir,
    MkCol,
    SyncUpload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebDavDirEntry {
    pub name: String,
    pub full_path: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebDavResult {
    pub op_id: u64,
    pub kind: WebDavOpKind,
    pub server_id: String,
    pub remote_path: String,
    pub ok: bool,
    pub status_code: u16,
    pub message: String,
    pub entries: Vec<WebDavDirEntry>,
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

    fn present_settings_frame(&mut self, frame: &WidgetFrame, inputs: SettingsInputRects);

    fn apply_settings_theme(&mut self, palette: FluentPalette);

    fn set_settings_anim_timer(&mut self, active: bool);

    fn settings_input_text(&self, slot: u8) -> String;

    fn set_settings_input_text(&self, slot: u8, text: &str);

    fn is_settings_window_open(&self) -> bool;

    fn settings_client_to_screen(&self, pt: PixelPoint) -> PixelPoint;

    fn show_fluent_popup_menu(
        &mut self,
        screen_pt: PixelPoint,
        spec: &FluentMenuSpec,
        palette: FluentPalette,
    ) -> Result<Option<u32>, String>;

    fn cursor_screen_pos(&self) -> PixelPoint;

    fn pick_sync_files(&mut self) -> Result<Vec<PathBuf>, String>;

    fn pick_sync_folder(&mut self) -> Result<Option<PathBuf>, String>;

    fn query_process_metrics(&mut self) -> ProcessMetrics;

    fn copy_to_clipboard(&self, text: &str) -> Result<(), String>;

    fn open_folder(&self, path: &Path) -> Result<(), String>;

    fn open_text_file(&self, path: &Path) -> Result<(), String>;

    fn is_autostart_enabled(&self) -> bool;

    fn set_autostart_enabled(&self, enabled: bool) -> Result<(), String>;

    fn cred_save(&self, target: &str, user: &str, secret: &str) -> Result<(), String>;

    fn cred_load(&self, target: &str) -> Result<Option<(String, String)>, String>;

    fn cred_delete(&self, target: &str) -> Result<(), String>;

    fn random_bytes(&self, len: usize) -> Result<Vec<u8>, String>;

    #[allow(clippy::too_many_arguments)]
    fn github_fetch_zipball(
        &self,
        op_id: u64,
        repo: &str,
        branch: &str,
        token: &str,
        dest_path: &Path,
        include_in_pending_backup: bool,
    ) -> Result<(), String>;

    fn webdav_test(
        &self,
        op_id: u64,
        server_id: &str,
        base_url: &str,
        user: &str,
        pass: &str,
    ) -> Result<(), String>;

    #[allow(clippy::too_many_arguments)]
    fn webdav_upload(
        &self,
        op_id: u64,
        server_id: &str,
        base_url: &str,
        remote_dir: &str,
        remote_name: &str,
        local_path: &Path,
        user: &str,
        pass: &str,
    ) -> Result<(), String>;

    #[allow(clippy::too_many_arguments)]
    fn webdav_list_dir(
        &self,
        op_id: u64,
        server_id: &str,
        base_url: &str,
        remote_dir: &str,
        user: &str,
        pass: &str,
    ) -> Result<(), String>;

    #[allow(clippy::too_many_arguments)]
    fn webdav_mkcol(
        &self,
        op_id: u64,
        server_id: &str,
        base_url: &str,
        remote_dir: &str,
        user: &str,
        pass: &str,
    ) -> Result<(), String>;

    #[allow(clippy::too_many_arguments)]
    fn webdav_sync_path(
        &self,
        op_id: u64,
        server_id: &str,
        base_url: &str,
        remote_dir: &str,
        local_path: &Path,
        user: &str,
        pass: &str,
    ) -> Result<(), String>;
}
