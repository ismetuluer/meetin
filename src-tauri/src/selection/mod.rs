//! Seçili metni okuma modülü (§4.1, M3.1).
//!
//! 1. Önce Windows UI Automation (`TextPattern::GetSelection`) ile panoya dokunmadan okur.
//! 2. Odaklanılan denetim parola kutusuysa (`IsPassword`) `None` döner.
//! 3. `TextPattern` desteklenmiyorsa panoyu yedekleyip `Ctrl+C` gönderir, okur ve geri yükler.

use crate::inject::clipboard;

/// Boş / yalnızca boşluk olan metni eler.
pub fn clean_selected_text(raw: Option<String>) -> Option<String> {
    raw.and_then(|s| {
        if s.trim().is_empty() {
            None
        } else {
            Some(s)
        }
    })
}

/// Etkin penceredeki seçili metni okur.
pub fn read_selected_text() -> Option<String> {
    #[cfg(windows)]
    {
        match win_sel::try_read_via_uia() {
            win_sel::UiaReadOutcome::PasswordField => None,
            win_sel::UiaReadOutcome::Text(t) => clean_selected_text(Some(t)),
            win_sel::UiaReadOutcome::Unsupported => {
                clean_selected_text(clipboard::read_selection_via_ctrl_c())
            }
        }
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Panodaki düz metni okur.
pub fn read_clipboard_text() -> Option<String> {
    clean_selected_text(clipboard::read_unicode_text())
}

#[cfg(windows)]
mod win_sel {
    use windows::core::Interface;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationTextPattern, UIA_IsPasswordPropertyId,
        UIA_TextPatternId,
    };

    pub enum UiaReadOutcome {
        PasswordField,
        Text(String),
        Unsupported,
    }

    pub fn try_read_via_uia() -> UiaReadOutcome {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let Ok(uia): Result<IUIAutomation, _> =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            else {
                return UiaReadOutcome::Unsupported;
            };
            let Ok(elem) = uia.GetFocusedElement() else {
                return UiaReadOutcome::Unsupported;
            };

            // Parola kutusundan ASLA metin okuma (§7.4)
            if let Ok(val) = elem.GetCurrentPropertyValue(UIA_IsPasswordPropertyId) {
                if let Ok(true) = bool::try_from(&val) {
                    return UiaReadOutcome::PasswordField;
                }
            }

            let Ok(pattern_unk) = elem.GetCurrentPattern(UIA_TextPatternId) else {
                return UiaReadOutcome::Unsupported;
            };
            let Ok(text_pattern): Result<IUIAutomationTextPattern, _> = pattern_unk.cast() else {
                return UiaReadOutcome::Unsupported;
            };
            let Ok(ranges) = text_pattern.GetSelection() else {
                return UiaReadOutcome::Unsupported;
            };
            let Ok(len) = ranges.Length() else {
                return UiaReadOutcome::Unsupported;
            };
            if len <= 0 {
                return UiaReadOutcome::Unsupported;
            }

            let mut combined = String::new();
            for i in 0..len {
                if let Ok(range) = ranges.GetElement(i) {
                    if let Ok(bstr) = range.GetText(-1) {
                        let part = bstr.to_string();
                        if !part.is_empty() {
                            if !combined.is_empty() {
                                combined.push('\n');
                            }
                            combined.push_str(&part);
                        }
                    }
                }
            }

            if combined.trim().is_empty() {
                UiaReadOutcome::Unsupported
            } else {
                UiaReadOutcome::Text(combined)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_whitespace_only_selections() {
        assert_eq!(clean_selected_text(None), None);
        assert_eq!(clean_selected_text(Some("   \n\t ".to_string())), None);
        assert_eq!(
            clean_selected_text(Some("Merhaba".to_string())),
            Some("Merhaba".to_string())
        );
    }
}
