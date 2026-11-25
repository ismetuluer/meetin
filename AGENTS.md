# AGENTS.md · Meetin

**Meetin:** Windows 10/11 için metin genişletici. Kısaltma + tetik tuşu → metin her programda tamamlanır. Yüzen koyun maskotu ve Dinamik Ada tarzı arayüz. Tauri 2 + React + TypeScript, çekirdek Rust. Arayüz Türkçe (`src/i18n/tr.json`), İngilizceye hazır.

## Kaynaklar

- `docs/MEETIN-SARTNAME.md` — tek doğru kaynak (şartnamede `SARTNAME.md` diye geçer). **[K]** değiştirilmez · **[Ö]** gerekçeyle daha iyisi önerilebilir · **[A]** gerçek ortamda denenecek.
- `docs/YOL_HARITASI.md` — adımlar ve durum. `CHANGELOG.md` — sürümler. `docs/HANDOFF.md` — yarım kalan iş (varsa).
- `docs/meetin-maskot.svg`, `docs/meetin-maskot-animasyon.html` — maskotun birebir kaynağı. Yeniden çizilmez; SVG yapısı, sınıf adları ve CSS animasyonları taşınır.
- Tasarım tuvali (yalnız görsel referans): https://claude.ai/artifact/XdbvzgCZMpGKceVg2XwBcR

## Klasör yapısı

```
src-tauri/src/  hook/ matcher/ inject/ selection/ vars/ db/ io/ windows/  (windows/ → Rust modülü `pencere`, `windows` crate ile karışmasın diye)
src/            mascot/ island/ bubble/ studio/ design/ i18n/
docs/           MEETIN-SARTNAME.md YOL_HARITASI.md HANDOFF.md
CHANGELOG.md    AGENTS.md
```

## Oturum başlangıcı (değişiklik yapmadan)

1. Bu dosya, `docs/YOL_HARITASI.md`, `CHANGELOG.md`, varsa `docs/HANDOFF.md` okunur. Şartnameden yalnız ilgili bölüm okunur.
2. Yol haritası durumu dosyalar ve `git log` ile doğrulanır; önceki raporlara güvenilmez.
3. Kısa durum özeti + sıradaki adımın planı (ajan, model, görev) verilir; Saul'un onayı beklenir.

## İş dağılımı

- **Haiku:** arama, log/test çıktısı okuma, `tr.json` metinleri, tek dosyada basit değişiklik. Rust çekirdeği, çok dosyalı iş ve veri güvenliği yasak.
- **Sonnet:** tek React bileşeni / tek modül, ada ve uzman arayüzü bileşenleri, maskot animasyonları, içe/dışa aktarma ayrıştırıcıları, renk/boşluk tutarlılığı.
- **Opus (ana oturum):** kanca, SendInput/pano, UI Automation, tr-TR harf eşleme, SQLite, güncelleme/dağıtım, palet üretici, mimari, güvenlik/gizlilik, alt ajan çıktısının denetimi.
- Ajan bağlamı baştan okuyacaksa ya da iş çok kısaysa Opus kendisi yapar.
- **Aynı anda tek alt ajan.** Her adımın başında ajan görevleri (ajan, model, görev) listelenir.
- Alt ajan kod değiştirdiyse sonuç gerçek dosyada ve testle doğrulanmadan teslim edilmez.

## Kurallar

- Bitti = zincirin tamamı çalışıyor: arayüz → etkileşim → iş mantığı → veri → hata durumu → geri bildirim → test.
- Teknik borç yok: sahte veri, ölü kod, çalışmayan düğme, sessizce yutulan hata, açıklamasız TODO olmaz. Bitmeyen iş `docs/HANDOFF.md`'ye yazılır.
- **Gizlilik:** Tuş vuruşu hiçbir zaman diske, loga ya da ağa yazılmaz. Arabellek yalnız bellekte, ≤ 64 karakter. Ağ yalnız GitHub güncelleme kontrolü için.
- Kanca geri çağrısı < 1 ms, asla bloklamaz. Çekirdekte `to_uppercase()`/`to_lowercase()` yerine tr-TR fonksiyonları kullanılır.
- Bileşenlerde sabit renk yok; yalnız §11.4 rolleri. Maskot renkleri (`--koyun-*`) sabit.
- Yeni bağımlılık için §10'daki liste dışındaysa gerekçe yazılır.
- Test edilmeyen şey "test edildi" diye raporlanmaz. Elle testler Saul'a açık adımlarla verilir.
- Bağlam dolmaya yaklaşınca `docs/HANDOFF.md`: durum, bitenler, bilinen hatalar, borç, sıradaki iş.

## Sürüm ve CHANGELOG

- SemVer. Aşama içi her adım yama artırır (`0.1.3`); aşama kabulü küçük sürüm çıkarır (M1 = `0.1.0` … M6 = `1.0.0`).
- Sürüm üç yerde eşit tutulur: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`.
- `CHANGELOG.md` Keep a Changelog biçiminde, Türkçe: `## [0.1.3] - YYYY-AA-GG` altında `Eklendi / Değişti / Düzeltildi / Kaldırıldı`. Yalnız gerçek, kullanıcıya görünür değişiklikler.
- Commit: `M1.3: SQLite katmanı ve v1 şeması` biçiminde; bir adım = bir (ya da birkaç küçük) commit. Aşama kabulünde `vX.Y.0` etiketi.
- Saul "Devam" derse: durumu anla → en önemli kalan işi seç → yap → test et → gerilemeyi kontrol et → sürüm + CHANGELOG → rapor + sıradaki adım.
