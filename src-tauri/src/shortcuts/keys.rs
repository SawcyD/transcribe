use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F10, VK_F11, VK_F12,
    VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7, VK_F8, VK_F9, VK_HOME, VK_INSERT, VK_LCONTROL,
    VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_NEXT, VK_PRIOR, VK_RCONTROL, VK_RETURN,
    VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};

/// Virtual-key codes are byte-sized, so a 256-entry table covers every key the
/// low-level hook can report.
pub const VK_COUNT: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Win,
}

impl Modifier {
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => Some(Modifier::Ctrl),
            "alt" | "menu" => Some(Modifier::Alt),
            "shift" => Some(Modifier::Shift),
            "win" | "meta" | "super" => Some(Modifier::Win),
            _ => None,
        }
    }

    /// Left, right, and generic variants all count as the modifier being held —
    /// Windows reports a mixture depending on the keyboard and driver.
    pub fn virtual_keys(self) -> &'static [u16] {
        match self {
            Modifier::Ctrl => &[VK_CONTROL.0, VK_LCONTROL.0, VK_RCONTROL.0],
            Modifier::Alt => &[VK_MENU.0, VK_LMENU.0, VK_RMENU.0],
            Modifier::Shift => &[VK_SHIFT.0, VK_LSHIFT.0, VK_RSHIFT.0],
            Modifier::Win => &[VK_LWIN.0, VK_RWIN.0],
        }
    }
}

/// Maps a user-facing key name to its virtual-key code.
pub fn key_to_vk(name: &str) -> Option<u16> {
    let upper = name.trim().to_ascii_uppercase();
    if upper.len() == 1 {
        let character = upper.as_bytes()[0];
        // 'A'-'Z' and '0'-'9' use their ASCII value as the virtual-key code.
        if character.is_ascii_uppercase() || character.is_ascii_digit() {
            return Some(u16::from(character));
        }
    }
    Some(match upper.as_str() {
        "SPACE" => VK_SPACE.0,
        "ESCAPE" | "ESC" => VK_ESCAPE.0,
        "ENTER" | "RETURN" => VK_RETURN.0,
        "TAB" => VK_TAB.0,
        "BACKSPACE" => VK_BACK.0,
        "DELETE" | "DEL" => VK_DELETE.0,
        "INSERT" => VK_INSERT.0,
        "HOME" => VK_HOME.0,
        "END" => VK_END.0,
        "PAGEUP" => VK_PRIOR.0,
        "PAGEDOWN" => VK_NEXT.0,
        "UP" => VK_UP.0,
        "DOWN" => VK_DOWN.0,
        "LEFT" => VK_LEFT.0,
        "RIGHT" => VK_RIGHT.0,
        "F1" => VK_F1.0,
        "F2" => VK_F2.0,
        "F3" => VK_F3.0,
        "F4" => VK_F4.0,
        "F5" => VK_F5.0,
        "F6" => VK_F6.0,
        "F7" => VK_F7.0,
        "F8" => VK_F8.0,
        "F9" => VK_F9.0,
        "F10" => VK_F10.0,
        "F11" => VK_F11.0,
        "F12" => VK_F12.0,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_and_digits_map_to_ascii() {
        assert_eq!(key_to_vk("b"), Some(0x42));
        assert_eq!(key_to_vk("B"), Some(0x42));
        assert_eq!(key_to_vk("5"), Some(0x35));
    }

    #[test]
    fn named_keys_are_case_insensitive() {
        assert_eq!(key_to_vk("space"), key_to_vk("SPACE"));
        assert_eq!(key_to_vk("esc"), key_to_vk("Escape"));
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert_eq!(key_to_vk("Fn"), None);
        assert_eq!(key_to_vk(""), None);
    }

    #[test]
    fn modifier_aliases_parse() {
        assert_eq!(Modifier::parse("Control"), Some(Modifier::Ctrl));
        assert_eq!(Modifier::parse("meta"), Some(Modifier::Win));
        assert_eq!(Modifier::parse("hyper"), None);
    }
}
