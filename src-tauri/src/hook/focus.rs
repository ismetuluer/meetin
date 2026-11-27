//! Odaklanan denetimin parola kutusu olup olmadığının izlenmesi (§7.4 [K]).
//!
//! Klavye kancası UI Automation çağıramaz (yavaş, süreçler arası). Bu yüzden:
//! 1. Odak ya da ön plan penceresi değişince WinEvent geri çağrısı [`FocusTracker::invalidate`]
//!    ile durumu `Bilinmiyor` yapar ve `meetin-focus` iş parçacığına istek yollar.
//! 2. O iş parçacığı UIA ile `IsPassword` sorgular, sonucu [`FocusTracker::resolve`] ile yazar.
//! 3. Kanca, durum `Normal` olmadıkça tuşa hiç dokunmaz: arabelleğe almaz, tetiği yutmaz.

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

/// Odaklanan denetimin bilinen durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FocusState {
    /// Odak değişti, sorgu sonucu henüz gelmedi.
    Unknown = 0,
    /// Parola kutusu değil; genişletme serbest.
    Normal = 1,
    /// Parola kutusu; kanca tam geçiş yapar.
    Password = 2,
}

impl FocusState {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Normal,
            2 => Self::Password,
            _ => Self::Unknown,
        }
    }

    /// UIA sorgusunun sonucunu duruma çevirir. Sorgu başarısızsa (`None`; ör. yönetici
    /// yetkili pencere, UIA desteklemeyen program) `Normal` sayılır: aksi hâlde böyle
    /// pencerelerde Meetin hiç çalışmazdı. Genişletme anındaki ikinci denetim yine yapılır.
    pub fn from_query(is_password: Option<bool>) -> Self {
        match is_password {
            Some(true) => Self::Password,
            Some(false) | None => Self::Normal,
        }
    }

    /// Kanca bu durumda tuşu hiç işlemeden geçirmeli mi?
    pub fn passes_through(self) -> bool {
        self != Self::Normal
    }
}

/// Kanca, WinEvent geri çağrısı ve odak iş parçacığı arasında kilitsiz paylaşılan durum.
#[derive(Debug)]
pub struct FocusTracker {
    state: AtomicU8,
    generation: AtomicU64,
}

impl Default for FocusTracker {
    fn default() -> Self {
        Self {
            state: AtomicU8::new(FocusState::Unknown as u8),
            generation: AtomicU64::new(0),
        }
    }
}

impl FocusTracker {
    pub fn state(&self) -> FocusState {
        FocusState::from_u8(self.state.load(Ordering::Acquire))
    }

    /// Odak değişti: durumu `Unknown` yapar ve yeni sorgu kuşağını döndürür.
    pub fn invalidate(&self) -> u64 {
        let generation = self.generation.fetch_add(1, Ordering::AcqRel) + 1;
        self.state
            .store(FocusState::Unknown as u8, Ordering::Release);
        generation
    }

    /// Sorgu başladığında geçerli kuşak.
    pub fn current_generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    /// Sorgu sonucunu yazar. Sorgu sürerken odak yeniden değiştiyse (kuşak ilerlediyse)
    /// sonuç eskidir ve yazılmaz; yeni istek ayrıca işlenecektir.
    pub fn resolve(&self, generation: u64, result: FocusState) -> bool {
        if self.current_generation() != generation {
            return false;
        }
        self.state.store(result as u8, Ordering::Release);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baslangic_bilinmiyor_ve_gecis_yapar() {
        let t = FocusTracker::default();
        assert_eq!(t.state(), FocusState::Unknown);
        assert!(t.state().passes_through());
    }

    #[test]
    fn yalniz_normal_durumda_islenir() {
        assert!(FocusState::Unknown.passes_through());
        assert!(FocusState::Password.passes_through());
        assert!(!FocusState::Normal.passes_through());
    }

    #[test]
    fn sorgu_sonucu_eslemesi() {
        assert_eq!(FocusState::from_query(Some(true)), FocusState::Password);
        assert_eq!(FocusState::from_query(Some(false)), FocusState::Normal);
        assert_eq!(FocusState::from_query(None), FocusState::Normal);
    }

    #[test]
    fn eski_sorgu_sonucu_yazilmaz() {
        let t = FocusTracker::default();
        let g1 = t.invalidate();
        let g2 = t.invalidate(); // sorgu sürerken odak yine değişti
        assert!(!t.resolve(g1, FocusState::Normal));
        assert_eq!(t.state(), FocusState::Unknown);
        assert!(t.resolve(g2, FocusState::Password));
        assert_eq!(t.state(), FocusState::Password);
    }

    #[test]
    fn odak_degisince_parola_durumu_sifirlanir() {
        let t = FocusTracker::default();
        let g = t.invalidate();
        t.resolve(g, FocusState::Password);
        t.invalidate();
        assert_eq!(t.state(), FocusState::Unknown);
        assert!(t.state().passes_through());
    }
}
