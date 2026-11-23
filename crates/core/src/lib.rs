pub mod ipc;
pub mod modules_cfg;

pub use ipc::{decode_supervisor_cmd, decode_worker_msg, encode_supervisor_cmd, encode_worker_msg};
pub use ipc::{SupervisorCmd, WorkerMsg};
pub use modules_cfg::{
    load_modules_config, parse_modules_config, save_modules_config, serialize_modules_config,
    IsolationMode, ModuleId, ModulesConfig,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleHealth {
    Stopped,
    Running { pid: Option<u32>, restarts: u32 },
    Faulted { restarts: u32 },
}

impl ModuleHealth {
    pub fn label(self) -> String {
        match self {
            Self::Stopped => "Отключён".to_owned(),
            Self::Running {
                pid: Some(p),
                restarts: 0,
            } => format!("Работает (PID {p})"),
            Self::Running {
                pid: Some(p),
                restarts,
            } => format!("Работает (PID {p}, рестартов: {restarts})"),
            Self::Running { pid: None, .. } => "Работает (в процессе)".to_owned(),
            Self::Faulted { restarts } => format!("Сбой (рестартов: {restarts})"),
        }
    }
}

pub trait Module {
    fn id(&self) -> ModuleId;
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
    fn health(&self) -> ModuleHealth;
}
