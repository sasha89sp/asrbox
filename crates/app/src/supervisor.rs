#![cfg(windows)]

use std::collections::{HashMap, VecDeque};
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use asrbox_core::{
    decode_worker_msg, encode_supervisor_cmd, IsolationMode, ModuleId, ModulesConfig,
    SupervisorCmd, WorkerMsg,
};
use asrbox_mod_settings::ModuleStatusView;
use asrbox_platform_windows::{wake_host_window, JobObject};

const MAX_RESTARTS_PER_MIN: usize = 3;

struct WorkerHandle {
    child: Child,
    stdin: Option<ChildStdin>,
    pid: u32,
}

#[derive(Default)]
struct RestartTracker {
    total: u32,
    recent: VecDeque<Instant>,
    halted_note: Option<String>,
}

pub enum SupervisorEvent {
    WorkerMsg(WorkerMsg),
    WorkerCrashed {
        id: ModuleId,
        code: Option<i32>,
        restarted: bool,
    },
    Error(String),
}

pub struct ModuleSupervisor {
    job: Option<JobObject>,
    workers: HashMap<ModuleId, WorkerHandle>,
    restarts: HashMap<ModuleId, RestartTracker>,
    inbox: Arc<Mutex<VecDeque<WorkerMsg>>>,
    host_hwnd_raw: usize,
    safe_mode: bool,
}

impl ModuleSupervisor {
    pub fn new(host_hwnd_raw: usize, safe_mode: bool) -> Self {
        let job = JobObject::new_kill_on_close().ok();
        Self {
            job,
            workers: HashMap::new(),
            restarts: HashMap::new(),
            inbox: Arc::new(Mutex::new(VecDeque::new())),
            host_hwnd_raw,
            safe_mode,
        }
    }

    pub fn sync_with_config(&mut self, cfg: &ModulesConfig) -> Vec<String> {
        let mut logs = Vec::new();
        for id in ModuleId::ALL {
            let want = cfg.isolation == IsolationMode::Process && cfg.is_enabled(id);
            if want && !self.workers.contains_key(&id) {
                if let Some(tr) = self.restarts.get_mut(&id) {
                    tr.halted_note = None;
                }
                match self.spawn_worker(id) {
                    Ok(pid) => logs.push(format!(
                        "Запущен изолированный процесс модуля {} (PID {pid})",
                        id.as_str()
                    )),
                    Err(err) => logs.push(format!("error: запуск модуля {}: {err}", id.as_str())),
                }
            } else if !want && self.workers.contains_key(&id) {
                self.stop_worker(id);
                logs.push(format!("Остановлен процесс модуля {}", id.as_str()));
            }
        }
        logs
    }

    pub fn restart_module(&mut self, id: ModuleId) -> Result<u32, String> {
        self.stop_worker(id);
        if let Some(tr) = self.restarts.get_mut(&id) {
            tr.halted_note = None;
            tr.recent.clear();
        }
        self.spawn_worker(id)
    }

    pub fn stop_all(&mut self) {
        for id in ModuleId::ALL {
            self.stop_worker(id);
        }
    }

    pub fn send_cmd(&mut self, id: ModuleId, cmd: &SupervisorCmd) -> bool {
        let Some(worker) = self.workers.get_mut(&id) else {
            return false;
        };
        let Some(stdin) = worker.stdin.as_mut() else {
            return false;
        };
        let line = encode_supervisor_cmd(cmd);
        writeln!(stdin, "{line}").is_ok() && stdin.flush().is_ok()
    }

    pub fn poll(&mut self, cfg: &ModulesConfig) -> Vec<SupervisorEvent> {
        let mut events = Vec::new();
        if let Ok(mut q) = self.inbox.lock() {
            while let Some(msg) = q.pop_front() {
                events.push(SupervisorEvent::WorkerMsg(msg));
            }
        }
        if cfg.isolation != IsolationMode::Process {
            return events;
        }
        let active_ids: Vec<ModuleId> = self.workers.keys().copied().collect();
        for id in active_ids {
            let exited = self
                .workers
                .get_mut(&id)
                .and_then(|w| w.child.try_wait().ok().flatten());
            let Some(status) = exited else {
                continue;
            };
            self.workers.remove(&id);
            if !cfg.is_enabled(id) {
                continue;
            }
            let restarted = self.maybe_restart_after_crash(id, &mut events);
            events.push(SupervisorEvent::WorkerCrashed {
                id,
                code: status.code(),
                restarted,
            });
        }
        events
    }

    pub fn status_views(&self, cfg: &ModulesConfig) -> Vec<ModuleStatusView> {
        ModuleId::ALL
            .into_iter()
            .map(|id| {
                let enabled = cfg.is_enabled(id);
                let tr = self.restarts.get(&id);
                let restarts = tr.map_or(0, |t| t.total);
                match cfg.isolation {
                    IsolationMode::Process => {
                        let pid = self.workers.get(&id).map(|w| w.pid);
                        let note = tr
                            .and_then(|t| t.halted_note.clone())
                            .unwrap_or_else(|| "не запущен".to_owned());
                        ModuleStatusView {
                            id,
                            enabled,
                            running: pid.is_some(),
                            pid,
                            restarts,
                            note,
                        }
                    }
                    IsolationMode::InProc => ModuleStatusView {
                        id,
                        enabled,
                        running: enabled,
                        pid: None,
                        restarts: 0,
                        note: "inproc".to_owned(),
                    },
                }
            })
            .collect()
    }

    fn maybe_restart_after_crash(
        &mut self,
        id: ModuleId,
        events: &mut Vec<SupervisorEvent>,
    ) -> bool {
        let now = Instant::now();
        let tr = self.restarts.entry(id).or_default();
        while let Some(&front) = tr.recent.front() {
            if now.duration_since(front) > Duration::from_secs(60) {
                tr.recent.pop_front();
            } else {
                break;
            }
        }
        if tr.recent.len() >= MAX_RESTARTS_PER_MIN {
            tr.halted_note = Some("остановлен после 3 сбоев за минуту".to_owned());
            return false;
        }
        tr.recent.push_back(now);
        tr.total = tr.total.saturating_add(1);
        match self.spawn_worker(id) {
            Ok(_) => true,
            Err(err) => {
                events.push(SupervisorEvent::Error(format!(
                    "не удалось перезапустить {}: {err}",
                    id.as_str()
                )));
                false
            }
        }
    }

    fn spawn_worker(&mut self, id: ModuleId) -> Result<u32, String> {
        let exe = env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
        let my_pid = std::process::id();
        let mut cmd = Command::new(exe);
        cmd.arg("--worker")
            .arg(id.as_str())
            .arg("--parent-pid")
            .arg(my_pid.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if self.safe_mode {
            cmd.arg("--safe-mode");
        }
        let mut child = cmd.spawn().map_err(|e| format!("spawn worker: {e}"))?;
        let pid = child.id();
        if let Some(job) = &self.job {
            let _ = job.assign_pid(pid);
        }
        let stdin = child.stdin.take();
        if let Some(stdout) = child.stdout.take() {
            let inbox = Arc::clone(&self.inbox);
            let hwnd_raw = self.host_hwnd_raw;
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    if let Some(msg) = decode_worker_msg(&line) {
                        if let Ok(mut q) = inbox.lock() {
                            q.push_back(msg);
                        }
                        wake_host_window(hwnd_raw);
                    }
                }
                wake_host_window(hwnd_raw);
            });
        }
        self.workers.insert(id, WorkerHandle { child, stdin, pid });
        Ok(pid)
    }

    fn stop_worker(&mut self, id: ModuleId) {
        if let Some(mut w) = self.workers.remove(&id) {
            if let Some(mut stdin) = w.stdin.take() {
                let _ = writeln!(stdin, "{}", encode_supervisor_cmd(&SupervisorCmd::Shutdown));
                let _ = stdin.flush();
            }
            let _ = w.child.kill();
            let _ = w.child.wait();
        }
    }
}

impl Drop for ModuleSupervisor {
    fn drop(&mut self) {
        self.stop_all();
    }
}
