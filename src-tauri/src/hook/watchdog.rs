//! Kanca nöbetçisi: Windows, yanıtı geciken düşük seviyeli kancayı haber vermeden
//! kaldırabilir. Bu durumda kullanıcı yazmaya devam eder ama kancamız hiç çağrılmaz.
//! Nöbetçi, sistemin son giriş zamanını (`GetLastInputInfo`) kancanın son çağrı zamanıyla
//! karşılaştırır; giriş olduğu halde kanca uzun süre çağrılmadıysa yeniden kurulmasını ister.
//!
//! Zamanlar `GetTickCount` milisaniyesidir (yaklaşık 49 günde bir başa döner; farklar
//! `wrapping_sub` ile alınır). Bu modül saf mantıktır; Windows çağrısı içermez.

/// Kancanın görmediği girişin "düşmüş kanca" sayılması için gereken en küçük fark.
pub const STALL_GAP_MS: u32 = 1_500;
/// Art arda kaç gözlemde şüphe sürmeli (tek seferlik gecikmeler yeniden kurulum tetiklemez).
pub const CONFIRM_OBSERVATIONS: u32 = 2;
/// İki yeniden kurulum arasındaki en kısa süre.
pub const COOLDOWN_MS: u32 = 30_000;

#[derive(Debug, Default)]
pub struct StallDetector {
    suspicious: u32,
    cooldown_until: Option<u32>,
}

impl StallDetector {
    pub const fn new() -> Self {
        Self {
            suspicious: 0,
            cooldown_until: None,
        }
    }

    /// Bir gözlem işler. `observable` yanlışsa (ör. ön plandaki pencere yönetici yetkili ve
    /// Windows girişini kancaya hiç göstermiyor) gözlem yok sayılır.
    /// Yeniden kurulum gerekiyorsa `true` döner.
    pub fn observe(&mut self, last_input: u32, last_hook: u32, now: u32, observable: bool) -> bool {
        if let Some(until) = self.cooldown_until {
            if (now.wrapping_sub(until) as i32) < 0 {
                return false;
            }
            self.cooldown_until = None;
        }
        let gap = last_input.wrapping_sub(last_hook) as i32;
        if !observable || gap <= STALL_GAP_MS as i32 {
            self.suspicious = 0;
            return false;
        }
        self.suspicious += 1;
        if self.suspicious < CONFIRM_OBSERVATIONS {
            return false;
        }
        self.suspicious = 0;
        self.cooldown_until = Some(now.wrapping_add(COOLDOWN_MS));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kanca_girisi_goruyorsa_tetiklenmez() {
        let mut d = StallDetector::new();
        for i in 0..10u32 {
            let t = 10_000 + i * 2_000;
            assert!(!d.observe(t, t - 5, t + 100, true));
        }
    }

    #[test]
    fn tek_gecikme_tetiklemez_suren_dusus_tetikler() {
        let mut d = StallDetector::new();
        assert!(!d.observe(20_000, 10_000, 20_100, true));
        assert!(d.observe(22_000, 10_000, 22_100, true));
    }

    #[test]
    fn arada_normal_gozlem_supheyi_sifirlar() {
        let mut d = StallDetector::new();
        assert!(!d.observe(20_000, 10_000, 20_100, true));
        assert!(!d.observe(22_000, 21_990, 22_100, true));
        assert!(!d.observe(24_000, 10_000, 24_100, true));
    }

    #[test]
    fn gozlenemeyen_durum_yok_sayilir() {
        let mut d = StallDetector::new();
        for i in 0..5u32 {
            let t = 20_000 + i * 2_000;
            assert!(!d.observe(t, 10_000, t + 100, false));
        }
    }

    #[test]
    fn bekleme_suresi_tekrar_tetiklemeyi_sinirlar() {
        let mut d = StallDetector::new();
        assert!(!d.observe(20_000, 10_000, 20_000, true));
        assert!(d.observe(22_000, 10_000, 22_000, true));
        assert!(!d.observe(24_000, 10_000, 24_000, true));
        assert!(!d.observe(26_000, 10_000, 26_000, true));
        // 30 s sonra yeniden değerlendirilir
        assert!(!d.observe(52_000, 10_000, 52_000, true));
        assert!(d.observe(54_000, 10_000, 54_000, true));
    }

    #[test]
    fn sayac_basa_donunce_dogru_calisir() {
        let mut d = StallDetector::new();
        let hook = u32::MAX - 500;
        assert!(!d.observe(hook.wrapping_add(100), hook, hook.wrapping_add(200), true));
        assert!(!d.observe(2_000, hook, 2_100, true));
        assert!(d.observe(4_000, hook, 4_100, true));
    }
}
