//! Pano yedeği için biçim sınıflandırma ve "gecikmeli yazma" bekleme kuralı (§1.3, §3.4).
//!
//! Saf mantık: Windows çağrısı yapmaz, birim testleriyle korunur.

use std::time::{Duration, Instant};

pub const CF_TEXT: u32 = 1;
pub const CF_BITMAP: u32 = 2;
pub const CF_METAFILEPICT: u32 = 3;
pub const CF_OEMTEXT: u32 = 7;
pub const CF_DIB: u32 = 8;
pub const CF_PALETTE: u32 = 9;
pub const CF_UNICODETEXT: u32 = 13;
pub const CF_ENHMETAFILE: u32 = 14;
pub const CF_DIBV5: u32 = 17;
pub const CF_OWNERDISPLAY: u32 = 0x80;
pub const CF_DSPBITMAP: u32 = 0x82;
pub const CF_DSPMETAFILEPICT: u32 = 0x83;
pub const CF_DSPENHMETAFILE: u32 = 0x8E;

/// Yedeklenecek toplam veri üst sınırı. Aşılırsa yalnız metin biçimleri alınır.
pub const MAX_BACKUP_BYTES: usize = 64 * 1024 * 1024;

/// Bir biçimin yedekte nasıl ele alınacağı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatKind {
    /// `HGLOBAL` bellek bloğu: baytları kopyalanır.
    Memory,
    /// Gelişmiş metafile tutamacı: `GetEnhMetaFileBits` ile kopyalanır.
    EnhMetafile,
    /// Atlanır: GDI tutamacı ya da Windows'un başka biçimden zaten türettiği biçim.
    Skip,
}

/// Panodaki biçim listesine bakarak tek bir biçimin nasıl yedekleneceğini söyler.
pub fn classify(fmt: u32, present: &[u32]) -> FormatKind {
    let has = |f: u32| present.contains(&f);
    match fmt {
        // Windows bunları CF_UNICODETEXT'ten (CF_LOCALE ile) yeniden türetir.
        CF_TEXT | CF_OEMTEXT if has(CF_UNICODETEXT) => FormatKind::Skip,
        // HBITMAP tutamacı; CF_DIB/CF_DIBV5'ten yeniden türetilir.
        CF_BITMAP => FormatKind::Skip,
        // İçinde HMETAFILE tutamacı taşır; CF_ENHMETAFILE'dan yeniden türetilir.
        CF_METAFILEPICT => FormatKind::Skip,
        CF_ENHMETAFILE => FormatKind::EnhMetafile,
        // GDI nesneleri ya da veri taşımayan biçimler.
        CF_PALETTE | CF_OWNERDISPLAY | CF_DSPBITMAP | CF_DSPMETAFILEPICT | CF_DSPENHMETAFILE => {
            FormatKind::Skip
        }
        // CF_PRIVATEFIRST..CF_GDIOBJLAST: içeriği tanımsız ya da GDI nesnesi.
        0x0200..=0x03FF => FormatKind::Skip,
        _ => FormatKind::Memory,
    }
}

/// Sınır aşıldığında yine de alınacak (küçük, metin) biçimler mi?
pub fn is_essential_text(fmt: u32) -> bool {
    matches!(
        fmt,
        CF_UNICODETEXT | CF_TEXT | CF_OEMTEXT | 16 /* CF_LOCALE */
    )
}

/// Hedef program panoyu ne zaman okuyup bitirdi? "Gecikmeli yazma"da her biçim istendiğinde
/// [`RenderWait::on_render`] çağrılır; [`RenderWait::done`] doğru olunca eski pano geri yüklenir.
#[derive(Debug, Clone)]
pub struct RenderWait {
    started: Instant,
    last_render: Option<Instant>,
    /// Hiç istek gelmezse en çok bu kadar beklenir (hedef yapıştırmadıysa).
    pub first_timeout: Duration,
    /// Son istekten sonra bu kadar sessizlik olursa okuma bitmiş sayılır
    /// (Word önce HTML, sonra RTF ister).
    pub quiet: Duration,
    /// Mutlak üst sınır.
    pub max_total: Duration,
}

impl RenderWait {
    pub fn new(now: Instant) -> Self {
        Self {
            started: now,
            last_render: None,
            first_timeout: Duration::from_millis(1500),
            quiet: Duration::from_millis(150),
            max_total: Duration::from_millis(2500),
        }
    }

    pub fn on_render(&mut self, now: Instant) {
        self.last_render = Some(now);
    }

    pub fn rendered(&self) -> bool {
        self.last_render.is_some()
    }

    pub fn done(&self, now: Instant) -> bool {
        if now.duration_since(self.started) >= self.max_total {
            return true;
        }
        match self.last_render {
            None => now.duration_since(self.started) >= self.first_timeout,
            Some(t) => now.duration_since(t) >= self.quiet,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metin_turetilmisleri_atlanir() {
        let p = [CF_UNICODETEXT, CF_TEXT, CF_OEMTEXT, 16];
        assert_eq!(classify(CF_UNICODETEXT, &p), FormatKind::Memory);
        assert_eq!(classify(CF_TEXT, &p), FormatKind::Skip);
        assert_eq!(classify(CF_OEMTEXT, &p), FormatKind::Skip);
        assert_eq!(classify(16, &p), FormatKind::Memory);
        // Unicode yoksa eski metin biçimi tek kaynaktır, korunur.
        assert_eq!(classify(CF_TEXT, &[CF_TEXT]), FormatKind::Memory);
    }

    #[test]
    fn resim_dib_ile_korunur_tutamac_atlanir() {
        let p = [CF_BITMAP, CF_DIB, CF_DIBV5, CF_PALETTE];
        assert_eq!(classify(CF_BITMAP, &p), FormatKind::Skip);
        assert_eq!(classify(CF_DIB, &p), FormatKind::Memory);
        assert_eq!(classify(CF_DIBV5, &p), FormatKind::Memory);
        assert_eq!(classify(CF_PALETTE, &p), FormatKind::Skip);
    }

    #[test]
    fn metafile_icerigiyle_alinir() {
        let p = [CF_ENHMETAFILE, CF_METAFILEPICT];
        assert_eq!(classify(CF_ENHMETAFILE, &p), FormatKind::EnhMetafile);
        assert_eq!(classify(CF_METAFILEPICT, &p), FormatKind::Skip);
    }

    #[test]
    fn ozel_ve_kayitli_bicimler() {
        assert_eq!(classify(0x0200, &[]), FormatKind::Skip);
        assert_eq!(classify(0x0300, &[]), FormatKind::Skip);
        // RegisterClipboardFormat aralığı (HTML Format, RTF, Excel biçimleri): bellek bloğu.
        assert_eq!(classify(0xC0A1, &[]), FormatKind::Memory);
    }

    #[test]
    fn istek_gelmezse_zaman_asimi() {
        let t = Instant::now();
        let w = RenderWait::new(t);
        assert!(!w.done(t + Duration::from_millis(1000)));
        assert!(w.done(t + Duration::from_millis(1500)));
    }

    #[test]
    fn son_istekten_sonra_sessizlik_bekler() {
        let t = Instant::now();
        let mut w = RenderWait::new(t);
        w.on_render(t + Duration::from_millis(40)); // HTML
        assert!(!w.done(t + Duration::from_millis(120)));
        w.on_render(t + Duration::from_millis(130)); // RTF
        assert!(!w.done(t + Duration::from_millis(250)));
        assert!(w.done(t + Duration::from_millis(280)));
    }

    #[test]
    fn mutlak_ust_sinir() {
        let t = Instant::now();
        let mut w = RenderWait::new(t);
        for ms in (0..2600).step_by(100) {
            w.on_render(t + Duration::from_millis(ms));
        }
        assert!(w.done(t + Duration::from_millis(2500)));
    }
}
