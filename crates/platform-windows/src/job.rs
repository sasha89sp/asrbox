use std::ffi::c_void;
use std::mem;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK,
};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA,
    PROCESS_TERMINATE,
};

const STILL_ACTIVE: u32 = 259;

/// Flags for the supervisor Job Object:
/// - `KILL_ON_JOB_CLOSE` ensures worker processes explicitly assigned to the job terminate when `asrbox` exits.
/// - `SILENT_BREAKAWAY_OK` ensures third-party apps launched by a dock worker automatically break away
///   from the job so closing `asrbox` or a dock never kills user-launched programs.
pub(crate) fn worker_job_limit_flags() -> JOB_OBJECT_LIMIT {
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK
}

pub struct JobObject {
    handle: HANDLE,
}

impl JobObject {
    pub fn new_kill_on_close() -> Result<Self, String> {
        // SAFETY: Creating an anonymous Win32 job object configured to terminate assigned workers on close
        // while allowing child applications spawned by workers to break away cleanly.
        unsafe {
            let handle = CreateJobObjectW(None, PCWSTR::null())
                .map_err(|e| format!("CreateJobObjectW failed: {e}"))?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = worker_job_limit_flags();
            let res = SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const c_void,
                u32::try_from(mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>()).unwrap_or(0),
            );
            if let Err(err) = res {
                let _ = CloseHandle(handle);
                return Err(format!("SetInformationJobObject failed: {err}"));
            }
            Ok(Self { handle })
        }
    }

    pub fn assign_pid(&self, pid: u32) -> Result<(), String> {
        // SAFETY: Opening child worker process handle solely to bind it to our job object, then closing it.
        unsafe {
            let proc = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid)
                .map_err(|e| format!("OpenProcess({pid}) failed: {e}"))?;
            let res = AssignProcessToJobObject(self.handle, proc);
            let _ = CloseHandle(proc);
            res.map_err(|e| format!("AssignProcessToJobObject({pid}) failed: {e}"))
        }
    }
}

impl Drop for JobObject {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            // SAFETY: Closing owned JobObject handle once on drop.
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

pub fn is_process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    // SAFETY: Querying exit code with limited read-only rights and closing handle immediately.
    unsafe {
        let Ok(proc) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };
        let mut code = 0u32;
        let ok = GetExitCodeProcess(proc, &mut code).is_ok();
        let _ = CloseHandle(proc);
        ok && code == STILL_ACTIVE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_flags_kill_workers_but_let_launched_apps_break_away() {
        let flags = worker_job_limit_flags();
        assert_ne!(flags.0 & JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE.0, 0);
        assert_ne!(flags.0 & JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK.0, 0);
    }
}
