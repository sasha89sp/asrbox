#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisorCmd {
    ReloadConfig,
    AddDock,
    ToggleDock(u64),
    RemoveDock(u64),
    ToggleCalendar,
    RefreshCalendar,
    ToggleSysMonitor,
    RefreshSysMonitor,
    SetOverlaySetupMode(bool),
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerMsg {
    Ready {
        module: String,
        pid: u32,
    },
    Log {
        level: String,
        module: String,
        message: String,
    },
    OpenSettings {
        tab: String,
    },
    ConfigChanged {
        module: String,
    },
    Metrics {
        module: String,
        ram_kb: u64,
        cpu_tenths: u32,
        gdi: u32,
        user: u32,
        events: u64,
        frames: u64,
        max_us: u64,
    },
}

pub fn encode_supervisor_cmd(cmd: &SupervisorCmd) -> String {
    match cmd {
        SupervisorCmd::ReloadConfig => "RELOAD".to_owned(),
        SupervisorCmd::AddDock => "ADD_DOCK".to_owned(),
        SupervisorCmd::ToggleDock(id) => format!("TOGGLE_DOCK|{id}"),
        SupervisorCmd::RemoveDock(id) => format!("REMOVE_DOCK|{id}"),
        SupervisorCmd::ToggleCalendar => "TOGGLE_CALENDAR".to_owned(),
        SupervisorCmd::RefreshCalendar => "REFRESH_CALENDAR".to_owned(),
        SupervisorCmd::ToggleSysMonitor => "TOGGLE_SYSMON".to_owned(),
        SupervisorCmd::RefreshSysMonitor => "REFRESH_SYSMON".to_owned(),
        SupervisorCmd::SetOverlaySetupMode(on) => {
            format!("OVERLAY_SETUP|{}", if *on { 1 } else { 0 })
        }
        SupervisorCmd::Shutdown => "SHUTDOWN".to_owned(),
    }
}

pub fn decode_supervisor_cmd(line: &str) -> Option<SupervisorCmd> {
    let t = line.trim();
    if t == "RELOAD" {
        return Some(SupervisorCmd::ReloadConfig);
    }
    if t == "ADD_DOCK" {
        return Some(SupervisorCmd::AddDock);
    }
    if t == "TOGGLE_CALENDAR" {
        return Some(SupervisorCmd::ToggleCalendar);
    }
    if t == "REFRESH_CALENDAR" {
        return Some(SupervisorCmd::RefreshCalendar);
    }
    if t == "TOGGLE_SYSMON" {
        return Some(SupervisorCmd::ToggleSysMonitor);
    }
    if t == "REFRESH_SYSMON" {
        return Some(SupervisorCmd::RefreshSysMonitor);
    }
    if t == "SHUTDOWN" {
        return Some(SupervisorCmd::Shutdown);
    }
    if let Some(rest) = t.strip_prefix("TOGGLE_DOCK|") {
        return rest.parse::<u64>().ok().map(SupervisorCmd::ToggleDock);
    }
    if let Some(rest) = t.strip_prefix("REMOVE_DOCK|") {
        return rest.parse::<u64>().ok().map(SupervisorCmd::RemoveDock);
    }
    if let Some(rest) = t.strip_prefix("OVERLAY_SETUP|") {
        return Some(SupervisorCmd::SetOverlaySetupMode(
            rest == "1" || rest.eq_ignore_ascii_case("true"),
        ));
    }
    None
}

pub fn encode_worker_msg(msg: &WorkerMsg) -> String {
    match msg {
        WorkerMsg::Ready { module, pid } => format!("READY|{module}|{pid}"),
        WorkerMsg::Log {
            level,
            module,
            message,
        } => {
            let safe = message.replace(['\r', '\n'], " ");
            format!("LOG|{level}|{module}|{safe}")
        }
        WorkerMsg::OpenSettings { tab } => format!("OPEN_SETTINGS|{tab}"),
        WorkerMsg::ConfigChanged { module } => format!("CONFIG_CHANGED|{module}"),
        WorkerMsg::Metrics {
            module,
            ram_kb,
            cpu_tenths,
            gdi,
            user,
            events,
            frames,
            max_us,
        } => format!(
            "METRICS|{module}|{ram_kb}|{cpu_tenths}|{gdi}|{user}|{events}|{frames}|{max_us}"
        ),
    }
}

pub fn decode_worker_msg(line: &str) -> Option<WorkerMsg> {
    let t = line.trim();
    if let Some(rest) = t.strip_prefix("READY|") {
        let (module, pid_s) = rest.split_once('|')?;
        return Some(WorkerMsg::Ready {
            module: module.to_owned(),
            pid: pid_s.parse().ok()?,
        });
    }
    if let Some(rest) = t.strip_prefix("LOG|") {
        let mut parts = rest.splitn(3, '|');
        let level = parts.next()?.to_owned();
        let module = parts.next()?.to_owned();
        let message = parts.next()?.to_owned();
        return Some(WorkerMsg::Log {
            level,
            module,
            message,
        });
    }
    if let Some(tab) = t.strip_prefix("OPEN_SETTINGS|") {
        return Some(WorkerMsg::OpenSettings {
            tab: tab.to_owned(),
        });
    }
    if let Some(module) = t.strip_prefix("CONFIG_CHANGED|") {
        return Some(WorkerMsg::ConfigChanged {
            module: module.to_owned(),
        });
    }
    if let Some(rest) = t.strip_prefix("METRICS|") {
        let cols: Vec<&str> = rest.split('|').collect();
        if cols.len() == 8 {
            return Some(WorkerMsg::Metrics {
                module: cols[0].to_owned(),
                ram_kb: cols[1].parse().ok()?,
                cpu_tenths: cols[2].parse().ok()?,
                gdi: cols[3].parse().ok()?,
                user: cols[4].parse().ok()?,
                events: cols[5].parse().ok()?,
                frames: cols[6].parse().ok()?,
                max_us: cols[7].parse().ok()?,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_roundtrip_for_supervisor_and_worker() {
        let cmds = [
            SupervisorCmd::ReloadConfig,
            SupervisorCmd::AddDock,
            SupervisorCmd::ToggleDock(3),
            SupervisorCmd::RemoveDock(2),
            SupervisorCmd::ToggleCalendar,
            SupervisorCmd::RefreshCalendar,
            SupervisorCmd::ToggleSysMonitor,
            SupervisorCmd::RefreshSysMonitor,
            SupervisorCmd::SetOverlaySetupMode(true),
            SupervisorCmd::SetOverlaySetupMode(false),
            SupervisorCmd::Shutdown,
        ];
        for c in cmds {
            assert_eq!(decode_supervisor_cmd(&encode_supervisor_cmd(&c)), Some(c));
        }

        let msgs = [
            WorkerMsg::Ready {
                module: "dock".to_owned(),
                pid: 4242,
            },
            WorkerMsg::Log {
                level: "INFO".to_owned(),
                module: "widgets".to_owned(),
                message: "hello | world".to_owned(),
            },
            WorkerMsg::OpenSettings {
                tab: "dock".to_owned(),
            },
            WorkerMsg::ConfigChanged {
                module: "dock".to_owned(),
            },
            WorkerMsg::Metrics {
                module: "dock".to_owned(),
                ram_kb: 8192,
                cpu_tenths: 5,
                gdi: 12,
                user: 8,
                events: 100,
                frames: 4,
                max_us: 850,
            },
        ];
        for m in msgs {
            assert_eq!(decode_worker_msg(&encode_worker_msg(&m)), Some(m));
        }
    }
}
