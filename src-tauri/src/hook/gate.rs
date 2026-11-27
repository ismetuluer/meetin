//! Genişletme sırasında kullanıcı tuşlarının sırasını koruyan kapı (§1.3).
//!
//! Worker metni yazarken (`SendInput` / pano) kullanıcı yazmaya devam ederse bu tuşlar
//! enjekte edilen metnin arasına karışabilir. Kapı açıkken kanca fiziksel tuş olaylarını
//! (basma ve bırakma) yutar ve sıraya alır; enjeksiyon bitince worker onları aynı sırayla
//! [`REPLAYED_EXTRA_INFO`](super::REPLAYED_EXTRA_INFO) imzasıyla yeniden gönderir.
//!
//! Kuyruk boşken kapanış, kuyrukla aynı kilit altında yapılır: kanca tuşu kuyruğa
//! koyduktan sonra worker'ın onu görmeden kapıyı kapatması mümkün değildir.
//!
//! Kapıyı tetiği yutan kanca açar ([`InjectGate::begin`]); böylece tetik ile worker'ın
//! işe başlaması arasında basılan tuşlar da beklemeye alınır. Yeniden gönderilen tuşlar
//! arasında yeni bir tetik bulunursa (art arda iki kısaltma), aynı partideki sonraki
//! tuşlar kuyruğun başına, sırası bozulmadan geri alınır ve ikinci genişletmeyi bekler.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Kuyruğa alınabilecek en fazla olay. Aşılırsa yeni tuşlar doğrudan geçer.
pub const MAX_QUEUED: usize = 256;
/// Kapının tuş tutabileceği en uzun süre. Takılan bir enjeksiyon klavyeyi kilitleyemez.
pub const MAX_HOLD: Duration = Duration::from_secs(3);

/// Yeniden gönderilecek tek bir tuş olayı (içinde metin değil, yalnız tuş kodu vardır).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueuedKey {
    pub vk: u16,
    pub scan: u16,
    pub up: bool,
    pub extended: bool,
}

#[derive(Debug, Default)]
struct Inner {
    open_since: Option<Instant>,
    queue: VecDeque<QueuedKey>,
    /// Worker şu an bir partiyi yeniden gönderiyor.
    replaying: bool,
    /// Yeniden gönderim sırasında yeni bir genişletme istendi.
    held_in_replay: bool,
    /// Geri alınan tuşların kuyruğun başında ekleneceği sıra.
    requeue_at: usize,
}

/// Kanca ile worker arasında paylaşılan kapı.
#[derive(Debug, Default)]
pub struct InjectGate {
    inner: Mutex<Inner>,
    /// Kilitsiz okuma için kapı durumu (fare kancası gibi yerler).
    active: AtomicBool,
}

impl InjectGate {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    /// Kapıyı açar. Kanca, worker'a enjeksiyon işi yollamadan hemen önce çağırır;
    /// worker'ın [`open`](Self::open) çağrısı aynı kapıyı sürdürür.
    pub fn begin(&self, now: Instant) {
        let mut g = self.lock();
        g.open_since.get_or_insert(now);
        if g.replaying && !g.held_in_replay {
            g.held_in_replay = true;
            g.requeue_at = 0;
        }
        self.active.store(true, Ordering::Release);
    }

    /// Kanca yeniden gönderilmiş bir tuş gördüğünde çağırır. Yeniden gönderim sırasında yeni
    /// bir genişletme istendiyse tuş kuyruğun başına (sırasıyla) geri alınır ve `true` döner.
    pub fn admit_replayed(&self, key: QueuedKey) -> bool {
        let mut g = self.lock();
        if !(g.replaying && g.held_in_replay) {
            return false;
        }
        let at = g.requeue_at;
        g.queue.insert(at, key);
        g.requeue_at += 1;
        true
    }

    /// Kanca her fiziksel tuş olayında çağırır. `true` dönerse olay yutulmalıdır.
    pub fn admit(&self, key: QueuedKey, now: Instant) -> bool {
        let mut g = self.lock();
        let Some(since) = g.open_since else {
            return false;
        };
        if now.duration_since(since) > MAX_HOLD || g.queue.len() >= MAX_QUEUED {
            return false;
        }
        g.queue.push_back(key);
        true
    }

    /// Worker enjeksiyondan sonra döngüde çağırır: sırada olay varsa onları döndürür
    /// (kapı açık kalır, yeniden gönderim sırasında gelenler de sıraya girer); sıra boşsa
    /// kapıyı kapatır ve `None` döner.
    ///
    /// Son partide yeni bir genişletme istendiyse kapı açık bırakılır ve `None` döner:
    /// kalan tuşlar o genişletmenin işi bitince gönderilir.
    pub fn take_or_close(&self) -> Option<Vec<QueuedKey>> {
        let mut g = self.lock();
        if g.held_in_replay {
            g.replaying = false;
            g.held_in_replay = false;
            g.requeue_at = 0;
            return None;
        }
        if g.queue.is_empty() {
            g.replaying = false;
            g.open_since = None;
            self.active.store(false, Ordering::Release);
            None
        } else {
            g.replaying = true;
            Some(g.queue.drain(..).collect())
        }
    }

    /// Kapıyı açar; dönen bekçi düşürülünce (panik dahil) sıradakiler `replay` ile
    /// gönderilir ve kapı kapanır.
    pub fn open<'a, F: FnMut(&[QueuedKey])>(&'a self, replay: F) -> GateGuard<'a, F> {
        self.begin(Instant::now());
        GateGuard { gate: self, replay }
    }
}

/// [`InjectGate::open`] bekçisi.
pub struct GateGuard<'a, F: FnMut(&[QueuedKey])> {
    gate: &'a InjectGate,
    replay: F,
}

impl<F: FnMut(&[QueuedKey])> Drop for GateGuard<'_, F> {
    fn drop(&mut self) {
        while let Some(batch) = self.gate.take_or_close() {
            (self.replay)(&batch);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(vk: u16, up: bool) -> QueuedKey {
        QueuedKey {
            vk,
            scan: 0,
            up,
            extended: false,
        }
    }

    #[test]
    fn kapali_kapi_tusu_gecirir() {
        let g = InjectGate::default();
        assert!(!g.admit(key(0x41, false), Instant::now()));
        assert!(!g.is_active());
    }

    #[test]
    fn acik_kapi_sirayi_korur() {
        let g = InjectGate::default();
        let t = Instant::now();
        g.begin(t);
        assert!(g.is_active());
        let seq = [
            key(0x41, false),
            key(0x41, true),
            key(0x42, false),
            key(0x42, true),
        ];
        for k in seq {
            assert!(g.admit(k, t));
        }
        assert_eq!(g.take_or_close().as_deref(), Some(&seq[..]));
        assert!(g.is_active(), "yeniden gönderim sürerken kapı açık kalır");
        assert_eq!(g.take_or_close(), None);
        assert!(!g.is_active());
    }

    #[test]
    fn yeniden_gonderim_sirasinda_gelenler_sonraki_partide() {
        let g = InjectGate::default();
        let t = Instant::now();
        g.begin(t);
        g.admit(key(1, false), t);
        let first = g.take_or_close().unwrap();
        g.admit(key(2, false), t); // ilk parti gönderilirken basıldı
        let second = g.take_or_close().unwrap();
        assert_eq!(first[0].vk, 1);
        assert_eq!(second[0].vk, 2);
        assert_eq!(g.take_or_close(), None);
    }

    #[test]
    fn kanca_actigi_kapiyi_worker_surdurur() {
        let g = InjectGate::default();
        let t = Instant::now();
        g.begin(t); // kanca tetiği yuttu
        assert!(g.admit(key(1, false), t)); // worker başlamadan basıldı
        let mut sent = Vec::new();
        {
            let _guard = g.open(|b: &[QueuedKey]| sent.extend_from_slice(b));
        }
        assert_eq!(sent, vec![key(1, false)]);
        assert!(!g.is_active());
    }

    #[test]
    fn art_arda_iki_genisletmede_sira_korunur() {
        // Kullanıcı "tpl␣ab" yazdı; ilk genişletme sürerken "␣ab" beklemede.
        let g = InjectGate::default();
        let t = Instant::now();
        g.begin(t);
        let sp = key(0x20, false);
        let a = key(0x41, false);
        let b = key(0x42, false);
        let c = key(0x43, false);
        for k in [sp, a, b] {
            g.admit(k, t);
        }
        assert_eq!(g.take_or_close(), Some(vec![sp, a, b]));
        g.admit(c, t); // gönderim sürerken fiziksel olarak basıldı
                       // Yeniden gönderilen boşluk bir tetikti: kanca yeni genişletme istedi.
        assert!(!g.admit_replayed(sp));
        g.begin(t);
        assert!(g.admit_replayed(a));
        assert!(g.admit_replayed(b));
        // Kapı açık kalır, kalanlar ikinci genişletmeyi bekler.
        assert_eq!(g.take_or_close(), None);
        assert!(g.is_active());
        // İkinci genişletme biter: a, b, c sırası korunur.
        assert_eq!(g.take_or_close(), Some(vec![a, b, c]));
        assert_eq!(g.take_or_close(), None);
        assert!(!g.is_active());
    }

    #[test]
    fn sure_asiminda_tus_gecer() {
        let g = InjectGate::default();
        let t = Instant::now();
        g.begin(t);
        assert!(!g.admit(key(1, false), t + MAX_HOLD + Duration::from_millis(1)));
    }

    #[test]
    fn kuyruk_siniri() {
        let g = InjectGate::default();
        let t = Instant::now();
        g.begin(t);
        for _ in 0..MAX_QUEUED {
            assert!(g.admit(key(1, false), t));
        }
        assert!(!g.admit(key(1, false), t));
    }

    #[test]
    fn bekci_panikte_de_kapatir_ve_gonderir() {
        let g = InjectGate::default();
        let mut sent = Vec::new();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = g.open(|b: &[QueuedKey]| sent.extend_from_slice(b));
            g.admit(key(7, false), Instant::now());
            panic!("enjeksiyon hatası");
        }));
        assert!(r.is_err());
        assert!(!g.is_active());
        assert_eq!(sent, vec![key(7, false)]);
    }
}
