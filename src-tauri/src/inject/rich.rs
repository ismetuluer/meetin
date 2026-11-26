//! Biçimli metin (CF_HTML + RTF + düz metin) dönüştürücüsü (§3.3, §3.4, M4.2).

/// HTML parçasını Windows `HTML Format` (CF_HTML v0.9) bayt dizisine sarar.
pub fn build_cf_html(fragment: &str) -> Vec<u8> {
    let prefix = "<html>\r\n<body>\r\n<!--StartFragment-->";
    let suffix = "<!--EndFragment-->\r\n</body>\r\n</html>";
    let body = format!("{prefix}{fragment}{suffix}");

    let header_template = "Version:0.9\r\n\
        StartHTML:0000000000\r\n\
        EndHTML:0000000000\r\n\
        StartFragment:0000000000\r\n\
        EndFragment:0000000000\r\n";

    let header_len = header_template.len();
    let start_html = header_len;
    let start_fragment = header_len + prefix.len();
    let end_fragment = start_fragment + fragment.len();
    let end_html = header_len + body.len();

    let header = format!(
        "Version:0.9\r\n\
        StartHTML:{start_html:010}\r\n\
        EndHTML:{end_html:010}\r\n\
        StartFragment:{start_fragment:010}\r\n\
        EndFragment:{end_fragment:010}\r\n"
    );

    let mut out = Vec::with_capacity(end_html + 1);
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(body.as_bytes());
    out.push(0); // Null sonlandırıcı
    out
}

/// HTML etiketlerini temizleyerek düz metin çıkarır (biçimsiz hedefler için).
pub fn html_to_plain_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();
    let mut list_stack: Vec<Option<usize>> = Vec::new();

    while let Some(c) = chars.next() {
        if c == '<' {
            let mut tag = String::new();
            for tc in chars.by_ref() {
                if tc == '>' {
                    break;
                }
                tag.push(tc);
            }
            let lower = tag.trim().to_ascii_lowercase();
            let name = lower
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches('/');
            match name {
                "ul" => list_stack.push(None),
                "ol" => list_stack.push(Some(0)),
                "/ul" | "/ol" => {
                    list_stack.pop();
                    if !out.ends_with('\n') && !out.is_empty() {
                        out.push('\n');
                    }
                }
                "br" | "/p" | "/div" | "/li" => {
                    if !out.ends_with('\n') && !out.is_empty() {
                        out.push('\n');
                    }
                }
                "li" => {
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push('\n');
                    }
                    match list_stack.last_mut() {
                        Some(Some(idx)) => {
                            *idx += 1;
                            out.push_str(&format!("{idx}. "));
                        }
                        _ => out.push_str("• "),
                    }
                }
                _ => {}
            }
        } else if c == '&' {
            let mut ent = String::new();
            while let Some(&nc) = chars.peek() {
                chars.next();
                if nc == ';' {
                    break;
                }
                ent.push(nc);
                if ent.len() > 8 {
                    break;
                }
            }
            match ent.as_str() {
                "amp" => out.push('&'),
                "lt" => out.push('<'),
                "gt" => out.push('>'),
                "quot" => out.push('"'),
                "#39" | "apos" => out.push('\''),
                "nbsp" => out.push(' '),
                _ => {
                    out.push('&');
                    out.push_str(&ent);
                    out.push(';');
                }
            }
        } else {
            out.push(c);
        }
    }

    out.trim_end_matches('\n').to_string()
}

fn push_rtf_escaped_text(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '\n' => out.push_str("\\line "),
            '\r' => {}
            '\t' => out.push_str("\\tab "),
            c if c.is_ascii() => out.push(c),
            c => {
                // RTF Unicode kaçışı (\uN?) — Türkçe karakterler dahil
                let mut buf = [0u16; 2];
                for unit in c.encode_utf16(&mut buf) {
                    let signed = *unit as i16;
                    out.push_str(&format!("\\u{signed}?"));
                }
            }
        }
    }
}

/// Basit biçimli HTML'yi Word/Outlook uyumlu RTF (`Rich Text Format`) baytlarına dönüştürür.
pub fn html_to_rtf(html: &str) -> Vec<u8> {
    let mut rtf = String::from(
        "{\\rtf1\\ansi\\ansicpg1254\\deff0{\\fonttbl{\\f0 Segoe UI;}}\\uc1\\f0\\fs22 ",
    );
    let mut list_stack: Vec<Option<usize>> = Vec::new();

    let mut rest = html;
    while !rest.is_empty() {
        if let Some(lt) = rest.find('<') {
            if lt > 0 {
                let plain_seg = decode_entities(&rest[..lt]);
                push_rtf_escaped_text(&mut rtf, &plain_seg);
            }
            let after_lt = &rest[lt + 1..];
            let Some(gt) = after_lt.find('>') else {
                push_rtf_escaped_text(&mut rtf, &rest[lt..]);
                break;
            };
            let raw_tag = after_lt[..gt].trim();
            let tag_name = raw_tag
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches('/')
                .to_ascii_lowercase();

            match tag_name.as_str() {
                "b" | "strong" => rtf.push_str("{\\b "),
                "/b" | "/strong" => rtf.push('}'),
                "i" | "em" => rtf.push_str("{\\i "),
                "/i" | "/em" => rtf.push('}'),
                "u" | "a" => rtf.push_str("{\\ul "),
                "/u" | "/a" => rtf.push('}'),
                "h1" | "h2" | "h3" => rtf.push_str("{\\b\\fs28 "),
                "/h1" | "/h2" | "/h3" => rtf.push_str("}\\par "),
                "br" => rtf.push_str("\\line "),
                "/p" | "/div" => rtf.push_str("\\par "),
                "ul" => list_stack.push(None),
                "ol" => list_stack.push(Some(0)),
                "/ul" | "/ol" => {
                    list_stack.pop();
                }
                "li" => match list_stack.last_mut() {
                    Some(Some(idx)) => {
                        *idx += 1;
                        rtf.push_str(&format!("{idx}. "));
                    }
                    _ => rtf.push_str("\\bullet  "),
                },
                "/li" => rtf.push_str("\\par "),
                _ => {}
            }

            rest = &after_lt[gt + 1..];
        } else {
            let plain_seg = decode_entities(rest);
            push_rtf_escaped_text(&mut rtf, &plain_seg);
            break;
        }
    }

    rtf.push('}');
    let mut bytes = rtf.into_bytes();
    bytes.push(0); // Null sonlandırıcı
    bytes
}

fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_valid_cf_html_with_exact_byte_offsets() {
        let frag = "<b>Merhaba İstanbul</b>";
        let bytes = build_cf_html(frag);
        let text = String::from_utf8_lossy(&bytes[..bytes.len() - 1]);

        let start_frag_line = text
            .lines()
            .find(|l| l.starts_with("StartFragment:"))
            .unwrap();
        let end_frag_line = text
            .lines()
            .find(|l| l.starts_with("EndFragment:"))
            .unwrap();
        let sf: usize = start_frag_line
            .split(':')
            .nth(1)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let ef: usize = end_frag_line
            .split(':')
            .nth(1)
            .unwrap()
            .trim()
            .parse()
            .unwrap();

        assert_eq!(&text.as_bytes()[sf..ef], frag.as_bytes());
    }

    #[test]
    fn converts_html_to_rtf_with_turkish_unicode_escapes() {
        let html = "<p><b>Kalın</b> ve <i>italik</i>: İğşüöç</p>";
        let rtf_bytes = html_to_rtf(html);
        let rtf = String::from_utf8_lossy(&rtf_bytes);

        assert!(rtf.starts_with("{\\rtf1"));
        assert!(rtf.contains("{\\b Kal\\u305?n}"));
        assert!(rtf.contains("{\\i italik}"));
        assert!(rtf.contains("\\u304?")); // 'İ'
        assert!(rtf.contains("\\par "));
    }

    #[test]
    fn extracts_plain_text_from_html() {
        let plain = html_to_plain_text("<p><b>Selam</b><br/>Dünya &amp; Ay</p>");
        assert_eq!(plain, "Selam\nDünya & Ay");
    }

    #[test]
    fn converts_ordered_and_unordered_lists_and_links() {
        let html = "<ul><li>Bir</li><li>İki</li></ul><ol><li>İlk</li><li>Son</li></ol><a href=\"https://example.com\">Bağlantı</a>";
        let plain = html_to_plain_text(html);
        assert_eq!(plain, "• Bir\n• İki\n1. İlk\n2. Son\nBağlantı");

        let rtf = String::from_utf8_lossy(&html_to_rtf(html)).into_owned();
        assert!(rtf.contains("\\bullet  Bir\\par "));
        assert!(rtf.contains("1. \\u304?lk\\par "));
        assert!(rtf.contains("2. Son\\par "));
        assert!(rtf.contains("{\\ul Ba\\u287?lant\\u305?}"));
    }
}
