//! Hedef programa metin yazma motoru (§3.4, M1.6, M1.7, M4.2, M4.3, M4.5).
//!
//! - Kısa düz metin (≤ 200 karakter): `SendInput` + `KEYEVENTF_UNICODE` (pano hiç kullanılmaz).
//! - Uzun veya biçimli metin: pano yedeklenir → CF_HTML + RTF + CF_UNICODETEXT yapıştırılır → pano geri yüklenir.
//! - `{imleç}` varsa genişletmeden sonra gereken sayıda `←` tuşu gönderilir.
//! - Windows Terminal (`wt.exe`, `windowsterminal.exe`) için `Ctrl+Shift+V` kullanılır.

pub mod clipboard;
pub mod rich;

use thiserror::Error;

use crate::matcher::case::tr_lower;
use crate::matcher::MatchResult;

use self::clipboard::PasteShortcut;

/// Kısa düz metin eşiği (§3.4).
pub const SHORT_TEXT_THRESHOLD: usize = 200;

/// Windows 11 WinUI 3 / TSF tabanlı uygulamalarda kısa metinlerde dahi
/// pano koruyarak yapıştırma kullanılan istisna listesi (§3.4).
pub const CLIPBOARD_PREFERRED_PROCESSES: &[&str] = &["notepad.exe"];

/// `Ctrl+Shift+V` ile yapıştırma isteyen terminal işlemleri (§3.4).
pub const CTRL_SHIFT_V_PROCESSES: &[&str] = &["windowsterminal.exe", "wt.exe"];

/// Hedef program için pano yapıştırma yöntemi seçilip seçilmeyeceğini belirler.
pub fn should_use_clipboard(text_char_len: usize, process_name: Option<&str>) -> bool {
    if text_char_len > SHORT_TEXT_THRESHOLD {
        return true;
    }
    if let Some(proc_name) = process_name {
        let lower = tr_lower(proc_name);
        if CLIPBOARD_PREFERRED_PROCESSES.iter().any(|&p| p == lower) {
            return true;
        }
    }
    false
}

/// Hedef programa uygun yapıştırma kısayolunu döndürür.
pub fn paste_shortcut_for_process(process_name: Option<&str>) -> PasteShortcut {
    if let Some(proc_name) = process_name {
        let lower = tr_lower(proc_name);
        if CTRL_SHIFT_V_PROCESSES.iter().any(|&p| p == lower) {
            return PasteShortcut::CtrlShiftV;
        }
    }
    PasteShortcut::CtrlV
}

#[derive(Debug, Error)]
pub enum InjectError {
    #[error("Pano şu an başka bir program tarafından kullanılıyor")]
    ClipboardBusy,
    #[error("Windows API hatası: {0}")]
    Win32(String),
}

/// Saf ve test edilebilir enjeksiyon adımı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectStep {
    Backspace,
    Enter,
    Tab,
    LeftArrow,
    UnicodeUnit(u16),
}

/// Kısa düz metin genişletmesi için gönderilecek olay dizisini üretir (saf fonksiyon).
pub fn build_short_text_sequence(
    abbr_char_len: usize,
    expanded: &str,
    trigger: char,
) -> Vec<InjectStep> {
    let mut steps = build_text_steps_without_trigger(abbr_char_len, expanded);

    // Tetik tuşunu sona ekle (§3.1)
    match trigger {
        '\n' | '\r' => steps.push(InjectStep::Enter),
        '\t' => steps.push(InjectStep::Tab),
        other => {
            let mut buf = [0u16; 2];
            for unit in other.encode_utf16(&mut buf) {
                steps.push(InjectStep::UnicodeUnit(*unit));
            }
        }
    }

    steps
}

fn build_text_steps_without_trigger(abbr_char_len: usize, text: &str) -> Vec<InjectStep> {
    let mut steps = Vec::with_capacity(abbr_char_len + text.len() + 2);
    for _ in 0..abbr_char_len {
        steps.push(InjectStep::Backspace);
    }
    for ch in text.chars() {
        match ch {
            '\n' => steps.push(InjectStep::Enter),
            '\r' => {}
            '\t' => steps.push(InjectStep::Tab),
            other => {
                let mut buf = [0u16; 2];
                for unit in other.encode_utf16(&mut buf) {
                    steps.push(InjectStep::UnicodeUnit(*unit));
                }
            }
        }
    }
    steps
}

/// `{imleç}` konumu için gereken sayıda `LeftArrow` adımını üretir (saf fonksiyon).
pub fn build_cursor_left_steps(left_count: usize) -> Vec<InjectStep> {
    vec![InjectStep::LeftArrow; left_count]
}

/// Değişkenleri ve `{imleç}` konumu çözümlenmiş ifadeyi (düz veya biçimli) hedef pencereye yazar.
pub fn inject_resolved(
    abbr_char_len: usize,
    plain: &str,
    html: Option<&str>,
    trigger: Option<char>,
    cursor_left_steps: usize,
    process_name: Option<&str>,
) -> Result<(), InjectError> {
    let shortcut = paste_shortcut_for_process(process_name);
    let char_len = plain.chars().count();
    let has_html = html.map(|h| !h.trim().is_empty()).unwrap_or(false);

    if has_html || should_use_clipboard(char_len, process_name) {
        if abbr_char_len > 0 {
            for _ in 0..abbr_char_len {
                send_steps(&[InjectStep::Backspace])?;
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            std::thread::sleep(std::time::Duration::from_millis(12));
        }

        if let Some(trig) = trigger {
            if trig != '\n' && trig != '\r' && trig != '\t' && !has_html {
                let mut full = plain.to_string();
                full.push(trig);
                clipboard::paste_rich_preserving_clipboard(&full, None, shortcut)?;
            } else {
                clipboard::paste_rich_preserving_clipboard(plain, html, shortcut)?;
                send_single_trigger(trig)?;
            }
        } else {
            clipboard::paste_rich_preserving_clipboard(plain, html, shortcut)?;
        }
    } else {
        if abbr_char_len > 0 {
            let backspaces = vec![InjectStep::Backspace; abbr_char_len];
            send_steps(&backspaces)?;
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let text_steps = match trigger {
            Some(trig) => build_short_text_sequence(0, plain, trig),
            None => build_text_steps_without_trigger(0, plain),
        };
        send_steps(&text_steps)?;
    }

    if cursor_left_steps > 0 {
        // Tetik eklendiyse imlecin tetikten de önce durması için +1 değil, metin içi konum korunur
        // (tetik varsa sona 1 karakter daha eklenmiş olduğu için 1 adım daha sola git).
        let extra = if trigger.is_some() { 1 } else { 0 };
        let arrows = build_cursor_left_steps(cursor_left_steps + extra);
        send_steps(&arrows)?;
    }

    Ok(())
}

/// Geriye dönük uyumlu düz metin genişletme sarmalayıcısı.
pub fn inject_expansion(
    matched: &MatchResult,
    process_name: Option<&str>,
) -> Result<(), InjectError> {
    inject_resolved(
        matched.abbr_char_len,
        &matched.expanded_text,
        matched.content_html.as_deref(),
        Some(matched.trigger),
        0,
        process_name,
    )
}

/// Arama/alan adasından seçilen ifadeyi Backspace göndermeden doğrudan etkin pencereye yazar.
pub fn inject_plain_text_direct(text: &str, process_name: Option<&str>) -> Result<(), InjectError> {
    if text.chars().count() == 0 {
        return Ok(());
    }
    inject_resolved(0, text, None, None, 0, process_name)
}

/// Genişletme hemen ardından Backspace ile geri alınırsa metni silip orijinal kısaltmayı geri yazar (§3.1, M4.3).
pub fn inject_undo(chars_to_delete: usize, restore_abbr: &str) -> Result<(), InjectError> {
    for _ in 0..chars_to_delete {
        send_steps(&[InjectStep::Backspace])?;
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    if !restore_abbr.is_empty() {
        let steps = build_text_steps_without_trigger(0, restore_abbr);
        send_steps(&steps)?;
    }
    Ok(())
}

/// Yalnızca tek bir tetik tuşunu gönderir (ör. parola kutusunda yutulan tuşu iletmek için).
pub fn send_single_trigger(trigger: char) -> Result<(), InjectError> {
    let steps = build_short_text_sequence(0, "", trigger);
    send_steps(&steps)
}

#[cfg(windows)]
fn send_steps(steps: &[InjectStep]) -> Result<(), InjectError> {
    use crate::hook::INJECTED_EXTRA_INFO;
    use std::mem::size_of;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
        VIRTUAL_KEY, VK_BACK, VK_LEFT, VK_RETURN, VK_TAB,
    };

    if steps.is_empty() {
        return Ok(());
    }

    let mut inputs = Vec::with_capacity(steps.len() * 2);

    for step in steps {
        match *step {
            InjectStep::Backspace => {
                push_vk(&mut inputs, VK_BACK, INJECTED_EXTRA_INFO);
            }
            InjectStep::Enter => {
                push_vk(&mut inputs, VK_RETURN, INJECTED_EXTRA_INFO);
            }
            InjectStep::Tab => {
                push_vk(&mut inputs, VK_TAB, INJECTED_EXTRA_INFO);
            }
            InjectStep::LeftArrow => {
                push_vk(&mut inputs, VK_LEFT, INJECTED_EXTRA_INFO);
            }
            InjectStep::UnicodeUnit(unit) => {
                inputs.push(INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: unit,
                            dwFlags: KEYEVENTF_UNICODE,
                            time: 0,
                            dwExtraInfo: INJECTED_EXTRA_INFO,
                        },
                    },
                });
                inputs.push(INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: unit,
                            dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: INJECTED_EXTRA_INFO,
                        },
                    },
                });
            }
        }
    }

    let sent = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        return Err(InjectError::Win32(format!(
            "SendInput eksik gönderdi: {sent}/{}",
            inputs.len()
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn push_vk(
    inputs: &mut Vec<windows::Win32::UI::Input::KeyboardAndMouse::INPUT>,
    vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY,
    extra: usize,
) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MapVirtualKeyW, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        MAPVK_VK_TO_VSC,
    };
    let scan = unsafe { MapVirtualKeyW(vk.0 as u32, MAPVK_VK_TO_VSC) as u16 };
    inputs.push(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: Default::default(),
                time: 0,
                dwExtraInfo: extra,
            },
        },
    });
    inputs.push(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: extra,
            },
        },
    });
}

#[cfg(not(windows))]
fn send_steps(_steps: &[InjectStep]) -> Result<(), InjectError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_exact_sequence_for_turkish_text_and_trigger() {
        let seq = build_short_text_sequence(4, "İğşü", ' ');
        // 4 Backspace + 4 Unicode harf + 1 boşluk = 9 adım
        assert_eq!(seq.len(), 9);
        assert_eq!(&seq[0..4], &[InjectStep::Backspace; 4]);
        assert_eq!(seq[4], InjectStep::UnicodeUnit('İ' as u16));
        assert_eq!(seq[5], InjectStep::UnicodeUnit('ğ' as u16));
        assert_eq!(seq[6], InjectStep::UnicodeUnit('ş' as u16));
        assert_eq!(seq[7], InjectStep::UnicodeUnit('ü' as u16));
        assert_eq!(seq[8], InjectStep::UnicodeUnit(' ' as u16));
    }

    #[test]
    fn handles_newlines_tabs_and_surrogate_pair_emoji() {
        let seq = build_short_text_sequence(2, "a\nb\t🐑", '\n');
        // 2 Backspace + 'a' + Enter + 'b' + Tab + 2 UTF-16 surrogate + Enter = 9
        assert_eq!(seq.len(), 9);
        assert_eq!(seq[3], InjectStep::Enter);
        assert_eq!(seq[5], InjectStep::Tab);
        assert_eq!(seq[8], InjectStep::Enter);
    }

    #[test]
    fn notepad_uses_clipboard_strategy_even_for_short_text() {
        assert!(should_use_clipboard(20, Some("Notepad.exe")));
        assert!(should_use_clipboard(20, Some("notepad.exe")));
        assert!(!should_use_clipboard(20, Some("WINWORD.EXE")));
        assert!(should_use_clipboard(250, Some("WINWORD.EXE")));
    }

    #[test]
    fn selects_ctrl_shift_v_for_windows_terminal_and_builds_left_arrows() {
        assert_eq!(
            paste_shortcut_for_process(Some("WindowsTerminal.exe")),
            PasteShortcut::CtrlShiftV
        );
        assert_eq!(
            paste_shortcut_for_process(Some("wt.exe")),
            PasteShortcut::CtrlShiftV
        );
        assert_eq!(
            paste_shortcut_for_process(Some("WINWORD.EXE")),
            PasteShortcut::CtrlV
        );
        assert_eq!(build_cursor_left_steps(3), vec![InjectStep::LeftArrow; 3]);
    }
}
