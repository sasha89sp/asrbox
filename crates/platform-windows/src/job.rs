use std::ffi::c_void;
use std::mem;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA,
    PROCESS_TERMINATE,
};

const STILL_ACTIVE: u32 = 259;

pub struct JobObject {
    handle: HANDLE,
}

impl JobObject {
    pub fn new_kill_on_close() -> Result<Self, String> {
        // SAFETY: Creating an anonymous Win32 job object configured to terminate child workers on close.
        unsafe {
            let handle = CreateJobObjectW(None, PCWSTR::null())
                .map_err(|e| format!("CreateJobObjectW failed: {e}"))?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
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
