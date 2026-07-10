pub fn is_terminal(process_name: Option<&str>) -> bool {
    matches!(
        process_name.map(str::to_ascii_lowercase).as_deref(),
        Some("windowsterminal.exe" | "powershell.exe" | "pwsh.exe" | "cmd.exe")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selects_terminal_profiles() {
        assert!(is_terminal(Some("WindowsTerminal.exe")));
        assert!(!is_terminal(Some("Code.exe")));
    }
}
