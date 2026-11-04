use windows::Win32::{
    Foundation::HWND,
    System::Threading::{AttachThreadInput, GetCurrentThreadId},
    UI::{
        Input::KeyboardAndMouse::SetFocus,
        WindowsAndMessaging::{
            BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
            IsWindow, SetForegroundWindow, ShowWindow, SW_RESTORE,
        },
    },
};

use crate::{errors::AppError, models::ActiveTarget};

pub fn restore(target: &ActiveTarget) -> Result<(), AppError> {
    unsafe {
        log::debug!(
            "restoring dictation target; process_id={}",
            target.process_id
        );
        let window = HWND(target.window_handle as *mut core::ffi::c_void);
        if !IsWindow(Some(window)).as_bool() {
            return Err(AppError::Insertion(
                "the original target window has closed".into(),
            ));
        }
        let target_thread = GetWindowThreadProcessId(window, None);
        let current_thread = GetCurrentThreadId();
        let attached = current_thread != target_thread
            && AttachThreadInput(current_thread, target_thread, true).as_bool();
        if IsIconic(window).as_bool() {
            let _ = ShowWindow(window, SW_RESTORE);
        }
        // SetForegroundWindow's return value is unreliable: it can report
        // failure while the window is (or becomes) foreground anyway, which used
        // to surface as a false "could not paste". Judge by the actual result
        // and give the foreground lock a couple of short retries.
        let mut focused = GetForegroundWindow() == window;
        for _ in 0..3 {
            if focused {
                break;
            }
            let _ = BringWindowToTop(window);
            let _ = SetForegroundWindow(window);
            std::thread::sleep(std::time::Duration::from_millis(15));
            focused = GetForegroundWindow() == window;
        }
        if !focused {
            if attached {
                let _ = AttachThreadInput(current_thread, target_thread, false);
            }
            return Err(AppError::Insertion(
                "Windows refused to restore focus to the target".into(),
            ));
        }
        if let Some(control) = target.control_handle {
            let control = HWND(control as *mut core::ffi::c_void);
            if IsWindow(Some(control)).as_bool() {
                let _ = SetFocus(Some(control));
            }
        }
        if attached {
            let _ = AttachThreadInput(current_thread, target_thread, false);
        }
        Ok(())
    }
}
