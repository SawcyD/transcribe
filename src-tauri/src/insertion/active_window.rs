use std::path::Path;

use windows::{
    core::PWSTR,
    Win32::{
        Foundation::{CloseHandle, HWND},
        System::Threading::{
            OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
            PROCESS_QUERY_LIMITED_INFORMATION,
        },
        UI::WindowsAndMessaging::{
            GetForegroundWindow, GetGUIThreadInfo, GetWindowTextLengthW, GetWindowTextW,
            GetWindowThreadProcessId, GUITHREADINFO,
        },
    },
};

use crate::{errors::AppError, models::ActiveTarget};

pub fn capture_active_target() -> Result<ActiveTarget, AppError> {
    unsafe {
        let window = GetForegroundWindow();
        if window.0.is_null() {
            return Err(AppError::Windows(
                "no foreground window is available".into(),
            ));
        }
        let mut process_id = 0u32;
        let thread_id = GetWindowThreadProcessId(window, Some(&mut process_id));
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        let control =
            if GetGUIThreadInfo(thread_id, &mut info).is_ok() && !info.hwndFocus.0.is_null() {
                Some(info.hwndFocus.0 as isize)
            } else {
                None
            };
        let title = window_text(window);
        let process_name = process_path(process_id).and_then(|path| {
            Path::new(&path)
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
        });
        if process_name
            .as_deref()
            .map(is_redacted_process)
            .unwrap_or(false)
        {
            return Err(AppError::Windows(
                "dictation is disabled for this privacy-sensitive application".into(),
            ));
        }
        let application_name = process_name.as_deref().map(application_display_name);
        Ok(ActiveTarget {
            window_handle: window.0 as isize,
            control_handle: control,
            process_id,
            application_name,
            process_name,
            window_title: title,
        })
    }
}

fn is_redacted_process(process: &str) -> bool {
    matches!(
        process.to_ascii_lowercase().as_str(),
        "credentialuibroker.exe"
            | "logonui.exe"
            | "keepass.exe"
            | "keepassxc.exe"
            | "1password.exe"
            | "bitwarden.exe"
            | "lastpass.exe"
    )
}

#[cfg(test)]
mod tests {
    use super::is_redacted_process;

    #[test]
    fn blocks_known_secret_managers() {
        assert!(is_redacted_process("KeePassXC.exe"));
        assert!(!is_redacted_process("Code.exe"));
    }
}

unsafe fn window_text(window: HWND) -> Option<String> {
    let length = GetWindowTextLengthW(window);
    if length <= 0 {
        return None;
    }
    let mut buffer = vec![0u16; length as usize + 1];
    let copied = GetWindowTextW(window, &mut buffer);
    if copied <= 0 {
        None
    } else {
        Some(String::from_utf16_lossy(&buffer[..copied as usize]))
    }
}

unsafe fn process_path(process_id: u32) -> Option<String> {
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id).ok()?;
    let mut buffer = vec![0u16; 32_768];
    let mut size = buffer.len() as u32;
    let result = QueryFullProcessImageNameW(
        process,
        PROCESS_NAME_FORMAT(0),
        PWSTR(buffer.as_mut_ptr()),
        &mut size,
    );
    let _ = CloseHandle(process);
    result.ok()?;
    Some(String::from_utf16_lossy(&buffer[..size as usize]))
}

fn application_display_name(process: &str) -> String {
    match process.to_ascii_lowercase().as_str() {
        "code.exe" => "Visual Studio Code".into(),
        "cursor.exe" => "Cursor".into(),
        "robloxstudiobeta.exe" => "Roblox Studio".into(),
        "discord.exe" => "Discord".into(),
        "chrome.exe" => "Google Chrome".into(),
        "msedge.exe" => "Microsoft Edge".into(),
        "windowsterminal.exe" => "Windows Terminal".into(),
        "powershell.exe" | "pwsh.exe" => "PowerShell".into(),
        "cmd.exe" => "Command Prompt".into(),
        "winword.exe" => "Microsoft Word".into(),
        _ => process.trim_end_matches(".exe").to_string(),
    }
}
