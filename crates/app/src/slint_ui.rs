#![cfg(windows)]

use std::collections::VecDeque;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use asrbox_platform_api::{PixelPoint, TrayMenuEntry, TrayModuleFlags};
use asrbox_platform_windows::{wake_host_window, JobObject};
use asrbox_ui_slint::{encode_runtime_status, encode_tray_show, TrayShowPayload, UiRuntimeStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiProcKind {
    Settings,
    Notes,
    Tray,
}

struct UiProcHandle {
    child: Child,
    stdin: Option<ChildStdin>,
}

pub(crate) struct SlintUiBridge {
    job: Option<JobObject>,
    settings: Option<UiProcHandle>,
    notes: Option<UiProcHandle>,
    tray: Option<UiProcHandle>,
    inbox: Arc<Mutex<VecDeque<(UiProcKind, String)>>>,
    host_hwnd_raw: usize,
}

impl SlintUiBridge {
    pub(crate) fn new(host_hwnd_raw: usize) -> Self {
        Self {
            job: JobObject::new_kill_on_close().ok(),
            settings: None,
            notes: None,
            tray: None,
            inbox: Arc::new(Mutex::new(VecDeque::new())),
            host_hwnd_raw,
        }
    }

    pub(crate) fn open_settings(
        &mut self,
        tab: i32,
        subtab: i32,
        status: &UiRuntimeStatus,
    ) -> Result<(), String> {
        self.reap_exited();
        if self.settings.is_some() {
            let tab_line = format!("TAB|{tab}|{subtab}");
            let status_line = encode_runtime_status(status);
            if send_line(&mut self.settings, &tab_line)
                && send_line(&mut self.settings, &status_line)
            {
                return Ok(());
            }
            stop_handle(&mut self.settings);
        }
        let args = [
            "--ui-settings".to_owned(),
            "--tab".to_owned(),
            tab.to_string(),
            "--subtab".to_owned(),
            subtab.to_string(),
        ];
        let mut handle = self.spawn_ui_process(UiProcKind::Settings, &args)?;
        let line = encode_runtime_status(status);
        if let Some(stdin) = handle.stdin.as_mut() {
            let _ = writeln!(stdin, "{line}");
            let _ = stdin.flush();
        }
        self.settings = Some(handle);
        Ok(())
    }

    pub(crate) fn send_settings_status(&mut self, status: &UiRuntimeStatus) {
        self.reap_exited();
        if self.settings.is_some() {
            let line = encode_runtime_status(status);
            if !send_line(&mut self.settings, &line) {
                stop_handle(&mut self.settings);
            }
        }
    }

    pub(crate) fn notify_notes_refresh(&mut self) {
        self.reap_exited();
        if self.notes.is_some() && !send_line(&mut self.notes, "REFRESH") {
            stop_handle(&mut self.notes);
        }
    }

    pub(crate) fn is_settings_open(&mut self) -> bool {
        self.reap_exited();
        self.settings.is_some()
    }

    pub(crate) fn is_notes_open(&mut self) -> bool {
        self.reap_exited();
        self.notes.is_some()
    }

    pub(crate) fn open_notes(&mut self) -> Result<(), String> {
        self.reap_exited();
        if self.notes.is_some() {
            if send_line(&mut self.notes, "REFRESH") {
                return Ok(());
            }
            stop_handle(&mut self.notes);
        }
        let args = ["--ui-notes".to_owned()];
        let handle = self.spawn_ui_process(UiProcKind::Notes, &args)?;
        self.notes = Some(handle);
        Ok(())
    }

    pub(crate) fn close_notes(&mut self) {
        stop_handle(&mut self.notes);
    }

    pub(crate) fn set_notes_visible(&mut self, visible: bool) -> Result<(), String> {
        if visible {
            self.open_notes()
        } else {
            self.close_notes();
            Ok(())
        }
    }

    pub(crate) fn prewarm_tray(&mut self) -> Result<(), String> {
        self.reap_exited();
        if self.tray.is_some() {
            return Ok(());
        }
        let args = ["--ui-tray".to_owned()];
        self.tray = Some(self.spawn_ui_process(UiProcKind::Tray, &args)?);
        Ok(())
    }

    pub(crate) fn show_tray_menu(
        &mut self,
        screen: PixelPoint,
        docks: &[TrayMenuEntry],
        flags: TrayModuleFlags,
    ) -> Result<(), String> {
        self.reap_exited();
        if self.tray.is_none() {
            self.prewarm_tray()?;
        }
        let line = encode_tray_show(&TrayShowPayload {
            x: screen.x,
            y: screen.y,
            flags,
            docks: docks.to_vec(),
        });
        if send_line(&mut self.tray, &line) {
            return Ok(());
        }
        stop_handle(&mut self.tray);
        self.prewarm_tray()?;
        if send_line(&mut self.tray, &line) {
            Ok(())
        } else {
            Err("failed to send SHOW to Slint tray process".to_owned())
        }
    }

    pub(crate) fn poll_events(&mut self) -> Vec<(UiProcKind, String)> {
        self.reap_exited();
        let mut out = Vec::new();
        if let Ok(mut q) = self.inbox.lock() {
            while let Some(ev) = q.pop_front() {
                out.push(ev);
            }
        }
        out
    }

    pub(crate) fn stop_all(&mut self) {
        stop_handle(&mut self.settings);
        stop_handle(&mut self.notes);
        stop_handle(&mut self.tray);
    }

    fn reap_exited(&mut self) {
        if self
            .settings
            .as_mut()
            .and_then(|h| h.child.try_wait().ok().flatten())
            .is_some()
        {
            self.settings = None;
        }
        if self
            .notes
            .as_mut()
            .and_then(|h| h.child.try_wait().ok().flatten())
            .is_some()
        {
            self.notes = None;
        }
        if self
            .tray
            .as_mut()
            .and_then(|h| h.child.try_wait().ok().flatten())
            .is_some()
        {
            self.tray = None;
        }
    }

    fn spawn_ui_process(
        &self,
        kind: UiProcKind,
        extra_args: &[String],
    ) -> Result<UiProcHandle, String> {
        let exe = env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
        let my_pid = std::process::id();
        let mut cmd = Command::new(exe);
        cmd.args(extra_args)
            .arg("--parent-pid")
            .arg(my_pid.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd.spawn().map_err(|e| format!("spawn slint ui: {e}"))?;
        if let Some(job) = &self.job {
            let _ = job.assign_pid(child.id());
        }
        let stdin = child.stdin.take();
        if let Some(stdout) = child.stdout.take() {
            let inbox = Arc::clone(&self.inbox);
            let hwnd_raw = self.host_hwnd_raw;
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    if !line.trim().is_empty() {
                        if let Ok(mut q) = inbox.lock() {
                            q.push_back((kind, line));
                        }
                        wake_host_window(hwnd_raw);
                    }
                }
                wake_host_window(hwnd_raw);
            });
        }
        Ok(UiProcHandle { child, stdin })
    }
}

impl Drop for SlintUiBridge {
    fn drop(&mut self) {
        self.stop_all();
    }
}

fn send_line(slot: &mut Option<UiProcHandle>, line: &str) -> bool {
    let Some(h) = slot.as_mut() else {
        return false;
    };
    let Some(stdin) = h.stdin.as_mut() else {
        return false;
    };
    writeln!(stdin, "{line}").is_ok() && stdin.flush().is_ok()
}

fn stop_handle(slot: &mut Option<UiProcHandle>) {
    if let Some(mut h) = slot.take() {
        if let Some(mut stdin) = h.stdin.take() {
            let _ = writeln!(stdin, "SHUTDOWN");
            let _ = stdin.flush();
        }
        let _ = h.child.kill();
        let _ = h.child.wait();
    }
}
