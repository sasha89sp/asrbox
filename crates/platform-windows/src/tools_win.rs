use std::path::{Path, PathBuf};

use asrbox_platform_api::{
    FluentMenuSpec, FluentPalette, PixelPoint, ProcessMetrics, SettingsInputRects, ToolsPlatform,
    WidgetFrame,
};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::autostart::{
    cred_delete, cred_load, cred_save, is_autostart_enabled, set_autostart_enabled,
};
use crate::fluent_menu::show_fluent_menu;
use crate::github_http::spawn_github_zipball;
use crate::settings_proc::{
    apply_settings_window_theme, is_settings_open, open_or_focus_settings_window, present_settings,
    read_settings_input_text, set_settings_window_anim_timer, settings_client_to_screen,
    settings_hwnd_or, write_settings_input_text,
};
use crate::shell::{
    copy_text_to_clipboard, open_in_explorer, open_in_notepad, pick_sync_files_dialog,
    pick_sync_folder_dialog, random_bytes_win,
};
use crate::tools_proc::{create_about_window, create_debug_window, ABOUT_UI, DEBUG_UI};
pub(crate) use crate::tools_state::{
    focus_window, scroll_edit_to_end, set_window_text, ToolsWinState,
};
use crate::webdav_http::{spawn_webdav_test, spawn_webdav_upload};
use crate::webdav_sync::{spawn_webdav_list_dir, spawn_webdav_mkcol, spawn_webdav_sync_path};
use crate::window::WindowsDockHost;

impl ToolsPlatform for WindowsDockHost {
    fn open_debug_window(&mut self, title: &str) -> Result<(), String> {
        if let Some(hwnd) = DEBUG_UI.with(|ui| ui.borrow().hwnd) {
            focus_window(hwnd);
            return Ok(());
        }
        let (ui_font, mono_font) = self.tools.ensure_fonts();
        self.tools.last_summary.clear();
        self.tools.last_logs.clear();
        self.tools.last_verbose = None;
        self.tools.last_file = None;
        self.tools.last_filter.clear();
        create_debug_window(self.hinstance, title, ui_font, mono_font)
    }

    fn update_debug_window(
        &mut self,
        summary: &str,
        logs: &str,
        verbose: bool,
        file_log: bool,
        filter_label: &str,
    ) {
        DEBUG_UI.with(|ui| {
            let st = ui.borrow();
            if st.hwnd.is_none() {
                return;
            }
            if self.tools.last_verbose != Some(verbose) {
                self.tools.last_verbose = Some(verbose);
                if let Some(h) = st.btn_verbose {
                    set_window_text(
                        h,
                        if verbose {
                            "TRACE: ВКЛ"
                        } else {
                            "TRACE: ВЫКЛ"
                        },
                    );
                }
            }
            if self.tools.last_file != Some(file_log) {
                self.tools.last_file = Some(file_log);
                if let Some(h) = st.btn_file {
                    set_window_text(
                        h,
                        if file_log {
                            "Файл: ВКЛ"
                        } else {
                            "Файл: ВЫКЛ"
                        },
                    );
                }
            }
            if self.tools.last_filter != filter_label {
                self.tools.last_filter = filter_label.to_owned();
                if let Some(h) = st.btn_filter {
                    set_window_text(h, &format!("Модуль: {filter_label}"));
                }
            }
            if self.tools.last_summary != summary {
                self.tools.last_summary = summary.to_owned();
                if let Some(h) = st.summary_edit {
                    set_window_text(h, summary);
                }
            }
            if self.tools.last_logs != logs {
                self.tools.last_logs = logs.to_owned();
                if let Some(h) = st.logs_edit {
                    set_window_text(h, logs);
                    scroll_edit_to_end(h);
                }
            }
        });
    }

    fn is_debug_window_open(&self) -> bool {
        DEBUG_UI.with(|ui| ui.borrow().hwnd.is_some())
    }

    fn open_about_window(&mut self, title: &str, content: &str) -> Result<(), String> {
        if let Some(hwnd) = ABOUT_UI.with(|ui| ui.borrow().hwnd) {
            ABOUT_UI.with(|ui| {
                if let Some(edit) = ui.borrow().text_edit {
                    set_window_text(edit, content);
                }
            });
            focus_window(hwnd);
            return Ok(());
        }
        let (ui_font, mono_font) = self.tools.ensure_fonts();
        create_about_window(self.hinstance, title, content, ui_font, mono_font)
    }

    fn open_settings_window(&mut self, title: &str, width: u32, height: u32) -> Result<(), String> {
        let (ui_font, _) = self.tools.ensure_fonts();
        open_or_focus_settings_window(self.hinstance, title, width, height, ui_font, self.theme)
    }

    fn present_settings_frame(&mut self, frame: &WidgetFrame, inputs: SettingsInputRects) {
        present_settings(frame, inputs);
    }

    fn apply_settings_theme(&mut self, palette: FluentPalette) {
        self.theme = palette;
        apply_settings_window_theme(palette);
    }

    fn set_settings_anim_timer(&mut self, active: bool) {
        set_settings_window_anim_timer(active);
    }

    fn settings_input_text(&self, slot: u8) -> String {
        read_settings_input_text(slot)
    }

    fn set_settings_input_text(&self, slot: u8, text: &str) {
        write_settings_input_text(slot, text);
    }

    fn is_settings_window_open(&self) -> bool {
        is_settings_open()
    }

    fn settings_client_to_screen(&self, pt: PixelPoint) -> PixelPoint {
        settings_client_to_screen(pt)
    }

    fn show_fluent_popup_menu(
        &mut self,
        screen_point: PixelPoint,
        spec: &FluentMenuSpec,
        palette: FluentPalette,
    ) -> Result<Option<u32>, String> {
        let owner = settings_hwnd_or(self.host_hwnd);
        show_fluent_menu(self.hinstance, owner, screen_point, spec, palette)
    }

    fn cursor_screen_pos(&self) -> PixelPoint {
        let mut pt = POINT::default();
        // SAFETY: Querying cursor screen position on UI thread.
        unsafe {
            let _ = GetCursorPos(&mut pt);
        }
        PixelPoint { x: pt.x, y: pt.y }
    }

    fn pick_sync_files(&mut self) -> Result<Vec<PathBuf>, String> {
        pick_sync_files_dialog(settings_hwnd_or(self.host_hwnd))
    }

    fn pick_sync_folder(&mut self) -> Result<Option<PathBuf>, String> {
        pick_sync_folder_dialog(settings_hwnd_or(self.host_hwnd))
    }

    fn is_autostart_enabled(&self) -> bool {
        is_autostart_enabled()
    }

    fn set_autostart_enabled(&self, enabled: bool) -> Result<(), String> {
        set_autostart_enabled(enabled)
    }

    fn cred_save(&self, target: &str, username: &str, secret: &str) -> Result<(), String> {
        cred_save(target, username, secret)
    }

    fn cred_load(&self, target: &str) -> Result<Option<(String, String)>, String> {
        cred_load(target)
    }

    fn cred_delete(&self, target: &str) -> Result<(), String> {
        cred_delete(target)
    }

    fn random_bytes(&self, len: usize) -> Result<Vec<u8>, String> {
        random_bytes_win(len)
    }

    fn github_fetch_zipball(
        &self,
        op_id: u64,
        repo: &str,
        branch: &str,
        token: &str,
        dest_path: &Path,
        include_in_pending_backup: bool,
    ) -> Result<(), String> {
        spawn_github_zipball(
            self.host_hwnd,
            op_id,
            repo.to_owned(),
            branch.to_owned(),
            token.to_owned(),
            dest_path.to_path_buf(),
            include_in_pending_backup,
        );
        Ok(())
    }

    fn webdav_test(
        &self,
        op_id: u64,
        server_id: &str,
        base_url: &str,
        user: &str,
        pass: &str,
    ) -> Result<(), String> {
        spawn_webdav_test(
            self.host_hwnd,
            op_id,
            server_id.to_owned(),
            base_url.to_owned(),
            user.to_owned(),
            pass.to_owned(),
        );
        Ok(())
    }

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
    ) -> Result<(), String> {
        spawn_webdav_upload(
            self.host_hwnd,
            op_id,
            server_id.to_owned(),
            base_url.to_owned(),
            remote_dir.to_owned(),
            remote_name.to_owned(),
            local_path.to_path_buf(),
            user.to_owned(),
            pass.to_owned(),
        );
        Ok(())
    }

    fn webdav_list_dir(
        &self,
        op_id: u64,
        server_id: &str,
        base_url: &str,
        remote_dir: &str,
        user: &str,
        pass: &str,
    ) -> Result<(), String> {
        spawn_webdav_list_dir(
            self.host_hwnd,
            op_id,
            server_id.to_owned(),
            base_url.to_owned(),
            remote_dir.to_owned(),
            user.to_owned(),
            pass.to_owned(),
        );
        Ok(())
    }

    fn webdav_mkcol(
        &self,
        op_id: u64,
        server_id: &str,
        base_url: &str,
        remote_dir: &str,
        user: &str,
        pass: &str,
    ) -> Result<(), String> {
        spawn_webdav_mkcol(
            self.host_hwnd,
            op_id,
            server_id.to_owned(),
            base_url.to_owned(),
            remote_dir.to_owned(),
            user.to_owned(),
            pass.to_owned(),
        );
        Ok(())
    }

    fn webdav_sync_path(
        &self,
        op_id: u64,
        server_id: &str,
        base_url: &str,
        remote_root: &str,
        local_path: &Path,
        user: &str,
        pass: &str,
    ) -> Result<(), String> {
        spawn_webdav_sync_path(
            self.host_hwnd,
            op_id,
            server_id.to_owned(),
            base_url.to_owned(),
            remote_root.to_owned(),
            local_path.to_path_buf(),
            user.to_owned(),
            pass.to_owned(),
        );
        Ok(())
    }

    fn query_process_metrics(&mut self) -> ProcessMetrics {
        self.metrics.sample()
    }

    fn copy_to_clipboard(&self, text: &str) -> Result<(), String> {
        copy_text_to_clipboard(self.host_hwnd, text)
    }

    fn open_folder(&self, path: &Path) -> Result<(), String> {
        open_in_explorer(path)
    }

    fn open_text_file(&self, path: &Path) -> Result<(), String> {
        open_in_notepad(path)
    }
}
