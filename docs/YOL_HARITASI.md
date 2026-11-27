# Meetin · Yol Haritası (ayağa kaldırma)

**Başlangıç noktası:** v1.3.1 (Gemini dönemi). Şartnamedeki M1–M6 kapsamı kodda büyük ölçüde var; ayrıntılı durum ve bulgular: `docs/DENETIM_GEMINI.md`. Eski M1–M6 adım listesi git geçmişinde (`f9ba2e7`).
**Sıra (Saul kararı, 27.11.2026):** önce güvenlik ve kararlılık → güncelleyici → arayüz sadeleştirme → bakım → kabul.
**Durum:** `[ ]` başlanmadı · `[~]` yarım · `[x]` tamamlandı (yalnız dosya + test ile doğrulanınca).
**Test:** *Oto* = `cargo test` / `npm test` · *Elle* = Saul'un yapacağı kontrol.
**Ajan:** Opus = ana oturum · Sonnet / Haiku = alt ajan (aynı anda tek). Her adımdan önce ajan görevleri listelenir, onay beklenir.

---

## R0 · Hazırlık

### [x] R0.1 · Gemini geçmişini çalışma klasörüne al, belgeleri eşitle

- AGENTS.md geri geldi, şartnameye Saul kararları işlendi, bu yol haritası yazıldı, yanıltıcı `ATLANANLAR_YOL_HARITASI.md` kaldırıldı.

---

## R1 · Güvenlik ve kararlılık → v1.4.0

### [x] R1.1 · Parola kutusunda tam geçiş — v1.3.2

- **Hedef:** Odak değişimi (`SetWinEventHook`, `EVENT_OBJECT_FOCUS`) worker'da izlenir, parola durumu atomik bayrakta tutulur. Kanca bu bayrak açıkken tuşu arabelleğe almaz, tetiği yutmaz, öneri göstermez (§7.4 [K]).
- **Modüller:** `hook/`
- **Bitti:** Parola kutusunda arabellek boş kalır; tetik hiç yutulmaz.
- **Test:** Oto: kanca karar fonksiyonu (parola bayrağı → geçiş) · Elle: Chrome'da parola alanına bir kısaltma + boşluk yaz (genişlememeli, sıra bozulmamalı), KeePass/Bitwarden alanı.
- **Ajan:** Opus.

### [x] R1.2 · Genişletme sırasında tuş sırası — v1.3.3

- **Hedef:** Enjeksiyon sürerken kullanıcının fiziksel tuşları yutulup kuyruğa alınır, enjeksiyon bitince aynı sırayla yeniden gönderilir (üst sınır + zaman aşımı ile kilitlenme önlenir).
- **Modüller:** `hook/`, `inject/`
- **Bitti:** 100+ kelime/dk yazarken genişletme sonrası harf kaçmaz, sıra bozulmaz.
- **Test:** Oto: kuyruk/sıralama saf fonksiyonu · Elle: Notepad ve Word'de hızlı yazma.
- **Ajan:** Opus.

### [x] R1.3 · Pano yedek ve geri yükleme — v1.3.4

- **Hedef:** GDI tutamaçlı biçimler (bitmap, metafile, palet) doğru yedeklenir ya da sentez biçimleri (DIB) üzerinden korunur; Windows'un türettiği biçimler çift yazılmaz. Geri yükleme sabit 95 ms yerine hedefin okumasını bekleyen uyarlanır gecikmeyle yapılır. Meetin'in kendi geri yüklemesi pano geçmişine düşmez.
- **Modüller:** `inject/clipboard.rs`
- **Bitti:** Panoda ekran görüntüsü, Excel aralığı ya da Word biçimli metin varken uzun ifade genişletilince pano aynen geri gelir.
- **Test:** Oto: biçim sınıflandırma · Elle: üç pano türüyle Word/Chrome'da uzun ifade, sonra `Ctrl+V`.
- **Ajan:** Opus.

### [x] R1.4 · Kayıt günlüğü ve çökme kaydı — v1.3.5

- **Hedef (Saul isteği):** Çökmelerin sebebi bulunabilsin. Veri klasöründe `logs\meetin.log`, boyutla dönen (ör. 5 × 1 MB). Kaydedilenler: açılış bilgisi (sürüm, Windows yapı no., kurulumlu/portable, WebView2 sürümü), hatalar ve uyarılar, Rust panikleri (mesaj + konum + backtrace), arayüzdeki yakalanmamış JS hataları, WebView2 süreç çökmesi, güncelleme/yedek/DB hataları. Ayarlar'da "Günlük klasörünü aç" düğmesi.
- **Asla kaydedilmez (§7.4 [K]):** tuş, tuş arabelleği, ifade metni, kısaltma, pano içeriği, pencere başlığı. Bunu korumak için günlük çağrıları yalnız sabit mesaj + sayı/kod alan küçük bir arayüzden yapılır; test bu kuralı denetler.
- **Modüller:** yeni `src-tauri/src/logging.rs`, `lib.rs` (panik kancası, açılış), ön yüz hata köprüsü, Ayarlar
- **Test:** Oto: dönme, panik kaydı, yasak içerik denetimi · Elle: bilerek tetiklenen test çöküşü sonrası günlük dosyası.
- **Ajan:** Opus (Rust tarafı) · Haiku (`tr.json`/`en.json` metinleri).

### [x] R1.5 · Portable "onedir" paketi — v1.3.6 · v1.3.7

- **Güncelleme (Saul kararı, v1.3.7):** Veri `%APPDATA%\Meetin`'de, portable yedekleri `data\backups`'ta; başka bilgisayarda yedekten geri yükleme önerisi. WebView2 gömülmez (aşağıdaki [A] kapandı); yoksa açılışta sorulup Microsoft'tan indirilir.

- **Hedef (Saul isteği):** Portable sürüm tek EXE değil, kendi klasörüyle gelir: `Meetin\Meetin.exe`, `Meetin\local-data.flag`, `Meetin\data\` (veriler, yedekler, günlükler), `CHANGELOG.md`. **Hata düzeltmesi:** bugünkü ZIP'te `local-data.flag` yok; portable sürüm verisini yanlışlıkla `%APPDATA%`'ya yazıyor. Eski kullanıcının `%APPDATA%\Meetin` verisi ilk açılışta (onayla) taşınabilir.
- **[A] Seçenek:** WebView2 "sabit sürüm" çalışma zamanı klasöre eklenebilir (WebView2'si olmayan/eski bilgisayarlarda da çalışır, paket ~180 MB büyür). Saul karar verir.
- **Modüller:** `scripts/package-portable.ps1`, `.github/workflows/release.yml`, `db/` (veri yolu)
- **Test:** Oto: veri yolu seçimi · Elle: ZIP'i USB'ye aç, çalıştır, verinin `data\`'da oluştuğunu gör; salt okunur klasörde anlaşılır hata.
- **Ajan:** Opus.

### [x] R1.6 · Kanca hızı — v1.3.7

- **Hedef:** Etkin pencere bilgisi (işlem adı, başlık) her tuşta değil, pencere değişince (`EVENT_SYSTEM_FOREGROUND`) güncellenir.
- **Modüller:** `hook/`
- **Bitti:** Kanca geri çağrısında ağır sistem çağrısı kalmaz.
- **Test:** Oto: geri çağrı süresi ölçümü (< 1 ms) · Elle: genel akıcılık.
- **Ajan:** Opus.

### [~] R1.7 · Olay tabanlı pano izleyici ve CSP — v1.3.8

- **Hedef:** 900 ms yoklama yerine `AddClipboardFormatListener`; `csp: null` yerine kısıtlı CSP (zengin düzenleyici ve görseller çalışmaya devam eder).
- **Modüller:** `inject/clipboard.rs`, `tauri.conf.json`
- **Test:** Oto: mevcut testler · Elle: boşta işlemci ~%0 (Görev Yöneticisi), pano geçmişi, düzenleyicide görsel.
- **Ajan:** Opus.

### [ ] R1.8 · R1 kabul — v1.4.0

- **Test:** Elle: Notepad, Word, Outlook, Chrome (WhatsApp Web, Gmail), Windows Terminal, kurum yazılımı; parola alanı; hızlı yazma; pano türleri. **Ajan:** Saul · Opus.

---

## R2 · Güncelleyici → v1.5.0

### [ ] R2.1 · İmza anahtarı (sürüm yok)

- **Hedef:** `tauri signer generate` ile anahtar çifti; özel anahtar ve parolası GitHub Secrets'a (**Saul elle ekler**), açık anahtar `tauri.conf.json`'a.
- **Ajan:** Opus (komutlar ve talimat) · Saul.

### [ ] R2.2 · `tauri-plugin-updater` geçişi — v1.4.1

- **Hedef:** Kendi yazılmış `updater/` (CNG ECDSA, PowerShell indirme/kurulum) kaldırılır; eklenti ile kontrol (açılışta + 24 saatte bir), maskot rozeti, CHANGELOG notları, "Şimdi kur / Sonra". İmzasız paket asla kurulmaz.
- **Modüller:** `updater/` (silinir), `lib.rs`, `src/studio/` güncelleme sayfası
- **Test:** Oto: sürüm karşılaştırma ve olay akışı · Elle: R2.5'te.
- **Ajan:** Opus.

### [ ] R2.3 · CI: imzalı yayın + köprü — v1.4.1

- **Hedef:** CI imzalı NSIS paketi ve eklentinin manifestini (`updater.json`) yayımlar. Eski istemciler (≤ 1.4.0) için eski biçimdeki `latest.json` bir süre daha yayımlanır ki yeni sürüme geçebilsinler.
- **Modüller:** `.github/workflows/release.yml`, `scripts/`
- **Ajan:** Opus.

### [ ] R2.4 · Portable güncelleme — v1.4.2

- **Hedef:** [A] Eklenti portable'ı kurmaz. Seçenekler: (a) portable sürüm ZIP'i indirip `data\`'ya dokunmadan EXE'yi değiştirir (şartname §9), (b) yalnız bildirim + indirme bağlantısı. Saul karar verir.
- **Ajan:** Opus.

### [ ] R2.5 · R2 kabul — v1.5.0

- **Test:** Elle: 1.3.1 (elle kurulu) → 1.4.x → 1.5.0 → test amaçlı 1.5.1; kurulumlu ve portable; bozuk imzalı paketin reddi. **Ajan:** Saul · Opus.

---

## R3 · Arayüz sadeleştirme → v1.6.0

Her adımdan önce mevcut ekranın görüntüsü ve önerilen düzen Saul'a gösterilir, onay sonrası uygulanır (şartname §6 "Liste ve düzenleyici sadeliği [K]").

**Hareket bileşenleri (Saul'un seçtiği designbookmark.com/motion şablonları):** Kaynak kod kopyalanmaz; React'te, §11.4 renk rolleri ve Segoe UI ile yeniden yazılır, yeni bağımlılık eklenmez (Tailwind/shadcn yok).

### [ ] R3.0 · Ortak yay (spring) altyapısı — v1.5.1

- **Hedef:** Şu an her animasyon ayrı `cubic-bezier` kullanıyor. `src/design/motion.ts` + tokenlar: sertlik/sönüm değerleri tek yerde (§11.5: 380/28; şablonların değerleri farklıysa Saul karar verir), `prefers-reduced-motion` desteği. Sonraki bileşenler bunu kullanır.
- **Ajan:** Opus (altyapı) · **Test:** Oto: yay çözücü birim testi.

### [ ] R3.0a · Aç/kapa anahtarı (Toggle switch) — Ayarlar'daki 15 onay kutusu yerine. **Ajan:** Sonnet · Opus denetim.

### [ ] R3.0b · Etiket girişi (Tag input) — düzenleyicideki etiket hapları. **Ajan:** Sonnet · Opus denetim.

### [ ] R3.0c · Kenar çubuğu daraltma (Sidebar collapse) — uzman arayüzünün sol çubuğu (bugün animasyonsuz). **Ajan:** Sonnet · Opus denetim.

### [ ] R3.0d · Yığın panel (Stack panel) — [A] yeri Saul'la belirlenecek (öneri: ifade sürüm geçmişi). **Ajan:** Sonnet · Opus denetim.

### [ ] R3.0e · Kısayollarda klavye tuşu görünümü (Kbd) — Saul isteği

- **Hedef:** `Ctrl+Alt+P` gibi düz yazılar yerine tuş biçiminde küçük kutucuklar (shadcn "Kbd" görünümü örnek alınır, kütüphane eklenmez): Yardım, Ayarlar › Kısayollar, komut paleti, ipuçları. Ortak `<Kbd>` / `<KbdGroup>` bileşeni; Windows adlandırması (Ctrl, Alt, Shift, Win, Boşluk, Enter ↵, Tab ⇥, oklar ←↑→↓), §11.4 renk rolleri.
- **Ajan:** Sonnet · Opus denetim. **Test:** Oto: kısayol metni → tuş listesi ayrıştırma · Elle.

### [ ] R3.0f · Maskot: monitörler arası fırlatma + roket — Saul isteği

- **Hedef:** (1) Fırlatma fiziği tek monitörle sınırlı (`get_mascot_monitor_bounds` → `current_monitor`, Rust'ta `clamp_to_monitor`). Tüm monitörlerin birleşimi kullanılır; zemin, maskotun altındaki monitöre göre hesaplanır; monitörler arası boşluğa düşerse en yakın monitöre yerleşir; farklı DPI ölçekleri doğru çevrilir. (2) Fırlatma hızı, maskotu başka bir monitöre taşıyacak kadar yüksekse **`rocket` animasyonu**: sırtta küçük bir roket ve alev izi, uçuş boyunca. Çizim referanstaki gibi değil, Meetin tarzında: kalın koyu kontur, sade şekiller, yumuşak renkler. Maskot renkleri değişmez, roket bir aksesuardır. (3) Kısa "fışş" roket sesi, fırlatma sesi ayarına bağlı. `prefers-reduced-motion` açıkken roket yok.
- **Şartname:** §5.1.1 kataloğuna `rocket` satırı [K] olarak eklenir.
- **Ajan:** Opus (çoklu monitör fiziği, Rust konumlandırma) · Sonnet (roket SVG + CSS animasyonu, Opus denetimi). **Test:** Oto: monitör birleşimi, zemin ve geçiş fiziği · Elle: iki monitörde (%100/%150) yavaş ve hızlı fırlatma.

### [ ] R3.0g · Mikro animasyonlar ("UI Moments" tarzı) — Saul isteği

- **Hedef:** `gif/` klasöründeki 72 örneğin tarzında (sade, tek renk, yaylı) küçük etkileşim animasyonları. Kod kopyalanmaz, ortak yay altyapısı (R3.0) ve §11.4 renk rolleriyle yeniden yazılır. Hangisinin nerede kullanılacağı Saul ile birlikte seçilir; seçilenler bu maddeye alt adım olarak eklenir.
- **Ajan:** Sonnet (bileşen başına ayrı görev, sırayla) · Opus denetim.

### [ ] R3.1 · İfade listesi — v1.5.2

- **Hedef:** Onay kutuları kaldırılır; `Ctrl`/`Shift` + tık çoklu seçim; ifadeler klasörlere sürükle-bırak; anlamsız simgeler çıkar; toplu işlemler sağ tık menüsünde.
- **Ajan:** Sonnet · Opus denetim. **Test:** Oto: seçim mantığı · Elle.

### [ ] R3.1a · Kaydet sonrası boş form ve ifade kopyalama — Saul isteği

- **Hedef:** "Yeni ifade" kaydedilince form boşalır, sıradaki ifade hemen yazılabilir (kaydedilen listede seçili görünür). İfade formunda "Kopyasını oluştur" simge düğmesi (ipuçlu): içerik, etiketler, klasör ve kurallar kopyalanır, kısaltma boş ya da çakışmasız öneriyle gelir.
- **Ajan:** Sonnet · Opus denetim (kısaltma çakışması, DB). **Test:** Oto: kopya oluşturma · Elle.

### [ ] R3.2 · Düzenleyici başlığı ve alan düzeni — v1.5.3

- **Hedef:** Seçimde üstte çıkan bilgi yazıları kaldırılır; kısayol ile klasör alanı ayrılır (A2); kural haplarında "sadece/hariç" görünür (A3).
- **Ajan:** Sonnet · Opus denetim. **Test:** Elle.

### [ ] R3.3 · Değişken hapları satır içi — v1.5.4

- **Hedef:** Düzenleyicide `{…}` ham yazısı görünmez, haplar metnin içindedir; listede `[Tarih (uzun)]` yerine okunur önizleme (A1).
- **Ajan:** Sonnet · Opus denetim. **Test:** Oto: HTML ↔ hap gidiş-dönüş · Elle.

### [ ] R3.4 · Dışa aktarma ve bağlam menüleri — v1.5.5

- **Hedef:** Tek "Dışa aktar…" girişi + biçim seçme penceresi (renkli/renksiz seçenek kalkar).
- **Ajan:** Sonnet. **Test:** Elle.

### [ ] R3.5 · Görünüm sayfası ve renk kuralı — v1.5.6

- **Hedef:** "Sistemle uyumlu" seçiliyken renk denetimleri pasif (A4); bileşenlerdeki sabit renkler role taşınır (A5).
- **Ajan:** Sonnet. **Test:** Oto: sabit renk taraması · Elle.

### [ ] R3.6 · R3 kabul — v1.6.0 · **Ajan:** Saul · Opus.

---

## R4 · Bakım → v1.7.0

### [ ] R4.1 · `lib.rs` komutlarını modüllere böl — Opus

### [ ] R4.2 · `StudioView.tsx` ve `MascotIslandApp.tsx`'i bileşenlere böl — Sonnet (dosya başına ayrı görev) · Opus denetim

- Davranış değişmez; her bölme mevcut testler + elle duman testiyle doğrulanır.

---

## Açık kararlar (Saul)

- R2.4: portable güncellemesi tam otomatik mi, yalnız bildirim mi?
- A6: fırlatma/çarpma sesleri varsayılan açık kalsın mı?
