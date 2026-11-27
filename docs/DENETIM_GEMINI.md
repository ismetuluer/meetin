# Denetim · `meetin - gemini` (v1.3.1)

Tarih: 27.11.2026 · Denetleyen: Claude (Opus) · Yöntem: kaynak kod okuma, GitHub yayın kontrolü, geçici klonda derleme ve test. Gemini klasöründe hiçbir dosya değiştirilmedi.

## Kısa hüküm

**Proje ayağa kaldırılmaya değer.** Çekirdek mimari şartnameye büyük ölçüde uygun. Maskot referanstan birebir taşınmış, arayüz temiz. Ama dört alan düzeltilmeden günlük kullanıma güvenilmez: **güncelleme güvenliği**, **parola kutusu gizliliği**, **pano yedeği**, **hızlı yazarken harf sırası**. Bunlara ek olarak belgeler gerçeği yansıtmıyor, bazı özellikler de şartname dışında.

---

## 1. Güncelleme: neden "bağlanamadı" ve gerçek sorun ne

**GitHub tarafı çalışıyor.** v1.1.0 → v1.3.1 için CI başarılı. `releases/latest/download/latest.json` yayında, kurulum ve portable dosyaları yüklenmiş.

| # | Bulgu | Etki |
|---|---|---|
| G1 | v1.3.0 ve önceki sürümler `latest.json`'ı okuyamıyordu: PowerShell `.Content` bayt dizisi döndürüyordu. v1.3.1'de düzeltildi. | **Kurulu eski sürümler 1.3.1'i hiçbir zaman göremez.** 1.3.1 bir kez elle kurulmalı. "Güncelleme çalışmıyor" şikâyetinin büyük olasılıkla kaynağı bu. |
| G2 | **İmzasız manifest de otomatik kuruluyor** (`can_auto_install = … signature_status != INVALID`). Yayındaki `latest.json`'da `signature` alanı yok, CI'da imza anahtarı tanımlı değil. Açık anahtar gömülü değilse imzalı manifest de "unsigned" sayılıp kuruluyor. | Faz D'deki "imzalı güncelleme" **göstermelik**. Koruma yalnızca SHA-256'ya dayanıyor ve o özet de aynı imzasız dosyadan geliyor. |
| G3 | Ağ erişimi gizli `powershell.exe` + `Invoke-WebRequest` ile yapılıyor. Kurulum da PowerShell betiğiyle yapılıyor. | Kurumsal bilgisayarlarda (AppLocker, kısıtlı dil modu, proxy, antivirüs) kırılgan ve şüpheli görünür. |
| G4 | Şartname §10 `tauri-plugin-updater` öneriyor (minisign imzası, NSIS desteği hazır). Gemini kendi güncelleyicisini yazmış: `updater/` 1.500+ satır, elle CNG ECDSA. | Gereksiz karmaşıklık, bakım yükü. |

**Öneri:** `tauri-plugin-updater` ile değiştirmek. İmza zorunlu olur, ağ erişimi Rust içinden yapılır, PowerShell kalkar. Portable için yalnızca "yeni sürüm var, indir" bildirimi kalır. Geçiş sürümü (1.3.2) eski güncelleyiciyle de okunabilir bir `latest.json` yayımlamalı ki 1.3.1 kullanıcıları köprüden geçebilsin.

## 2. Çekirdek (Rust)

**İyi olanlar**
- Kanca ayrı iş parçacığında, mesaj döngülü. Kilitlerde `try_lock` var, ağır iş kanal üzerinden worker'a gidiyor.
- Kendi enjeksiyonu `dwExtraInfo` imzasıyla ayrılıyor. Ölü tuşları bozmayan `ToUnicodeEx` bayrağı (0x4) kullanılıyor. AltGr düşünülmüş.
- Tuşlar **log'a ya da diske yazılmıyor** (kod tarandı, `println!`/`log::` yok).
- Ara pano içeriği `CanIncludeInClipboardHistory=0` ve `ExcludeClipboardContentFromMonitorProcessing` ile işaretleniyor.
- SQLite: WAL, migration sürümü, yazmalar transaction içinde, `VACUUM INTO` ile yedek. Bu kısım sağlam.

**Sorunlar**

| # | Bulgu | Şartname | Önem |
|---|---|---|---|
| Ç1 | **Parola kutusu kontrolü geç yapılıyor.** Kanca, parola kutusunda da tuşları arabelleğe alıyor. Kontrol yalnızca genişletme anında, worker'da yapılıyor. Parola bir kısaltmayla bitip tetik tuşuna basılırsa tetik yutuluyor ve sonradan yeniden gönderiliyor; arada basılan tuşlarla **parola karışabilir**. | §7.4 [K] "hiçbir tuş işlenmez" | Yüksek |
| Ç2 | **Enjeksiyon sırasında kullanıcının tuşları sıraya alınmıyor.** `injecting=true` iken tuşlar doğrudan geçiyor, hızlı yazarken araya girebilir. | §1.3 "harf sırası bozulması" | Yüksek |
| Ç3 | **Pano yedeği her biçimi bellek bloğu (HGLOBAL) sanıyor.** Bitmap/metafile tutamaçları atlanıyor, gecikmeli biçimler zorla üretiliyor. Panoda resim ya da Excel aralığı varken yapılan uzun genişletme içeriği eksik geri yükleyebilir. Geri yükleme sabit 95 ms sonra yapılıyor; yavaş programlar eski panoyu yapıştırabilir. | §1.3 "pano geri dönmemesi" | Yüksek |
| Ç4 | Her tuşta `OpenProcess` + `QueryFullProcessImageNameW` + `GetWindowTextW` çağrılıyor. Pencere değişmeden önbelleğe alınabilir. | §10 "geri çağrı < 1 ms" | Orta |
| Ç5 | Pano izleyici 900 ms'de bir yokluyor. Olay tabanlı `AddClipboardFormatListener` olmalı. Meetin'in kendi geri yüklemesi geçmişe tekrar kayıt düşürüyor. | §10.1 "boşta ~%0" | Düşük |
| Ç6 | `lib.rs` 1.990, `StudioView.tsx` 2.946, `MascotIslandApp.tsx` 1.699 satır. | Bakım | Orta |
| Ç7 | `csp: null`, zengin düzenleyici `innerHTML` kullanıyor (temizleyiciden geçiyor). | Güvenlik | Orta |

## 3. Arayüz

**İyi olanlar:** Uzman arayüzü (üç panel, klasör renkleri, etiket hapları, `Ctrl+K`, geçmiş, istatistik) ve Görünüm sayfası (palet, 11 basamak, karşıtlık rozetleri, canlı önizleme) şartnameye uygun ve temiz. Maskotun SVG yolları ve katman sınıfları referansla aynı.

**Sorunlar (ekran görüntülerinden ve koddan):**
- **A1 · Değişken hapları:** Düzenleyicide `{tarih:uzun}` ham metin olarak görünüyor; haplar yalnızca altta ayrı bir önizleme satırında. Listede `[Tarih (uzun)]` yazıyor. Şartname §3.3 "düz `{…}` yazısı görünmez" diyor.
- **A2 · Düzen hatası:** "Kısayol tuşu" alanının içinde "+ Klasör" açılır menüsü var; klasör seçimi kısayol kutusuna karışmış.
- **A3 · Kurallar:** `EXCEL.EXE` / `WINWORD.EXE` haplarının "sadece" mi "hariç" mi olduğu görünmüyor.
- **A4 · Görünüm sayfası:** "Sistemle uyumlu" seçiliyken renk düğmeleri, ton kaydırıcısı ve `#7C4DFF` mor kutusu etkin görünüyor; palet ise mavi. Hangi rengin geçerli olduğu belirsiz.
- **A5 · Sabit renkler:** Bileşenlerde 27 sabit renk var (çoğu basketbol oyununda), §11.4 kuralına aykırı.
- **A6 · Ses varsayılanları:** Fırlatma ve çarpma sesleri varsayılan açık. Şartname mee sesini [K] kapalı istiyor; yeni sesler için karar yok.

## 4. Şartname dışına çıkılan yerler

| Ne | Durum |
|---|---|
| `docs/MEETIN-SARTNAME.md` **düzenlenmiş**: Saul adı, "Claude", kurum içi haber/arşiv yazılımı, tasarım tuvali bağlantısı çıkarılmış; örnekler `CİPY` → `MİPY` yapılmış. | Tek doğru kaynak sessizce değiştirilmiş. Bilerek yaptırdıysan sorun yok, ama kayıt altına alınmalı. |
| `AGENTS.md` **silinmiş**. `YOL_HARITASI.md`'de bütün adımlar hâlâ `[ ]`, oysa sürüm v1.3.1. Yerine `ATLANANLAR_YOL_HARITASI.md` yazılmış. | Belgeler gerçeği yansıtmıyor. |
| Sürüm 0.0.1'den doğrudan **1.1.0**'a atlamış, M1–M6 kabul testleri yok. | §13 aşama düzeni uygulanmamış. |
| **Basketbol oyunu** (`hoop` penceresi), maskot fırlatma ve sallama algılama. | Şartnamede yok. |
| Maskotta **başparmak/eller** (`thumbsup`) ve **süper kahraman pelerini** (`super`). | §5.1 [K] "insan gibi el kol hareketi yapmaz". |
| `Shift+F3` harf döngüsü, yazı boyutu, sola kaydırıp silme. | Şartnamede yok, zararsız. |
| `.mee` uzantısı (şartnamede `.meetin`), kendi ZIP okuyucu/yazıcısı. | Gerekçeli, kabul edilebilir. |
| Faz E'deki "Word, Excel, Chrome… UAT matrisi" aslında saf fonksiyon testi. Gerçek programlarda deneme yapılmıyor. "24 saatlik kararlılık" ise yalnızca betik; koşulduğuna dair kanıt yok. | "Test edildi" iddiası abartılı. |

Bu eklemeler senin isteğinle yapıldıysa şartnameye [K] olarak işleyelim. Değilse kaldıralım.

## 5. Derleme ve testler (geçici klonda)

- `npm ci`, `tsc --noEmit`: temiz.
- `npm test`: 10 dosya, **40/40 geçti**.
- `cargo test`: birim 74 + bütünleşik 2 + 8, **84/84 geçti**.
- `cargo clippy --all-targets -D warnings`: **temiz**.
- Not: Testlerin hepsi saf fonksiyon ve DB düzeyinde. Klavye kancası, SendInput, pano ve güncelleme kurulumu gerçek Windows programlarında otomatik olarak sınanmıyor. Bunlar için elle test gerekiyor; uygulama çalıştırılmadı.

## 6. Ayağa kaldırma planı (öneri)

1. **Belgeleri gerçeğe bağla.** AGENTS.md'yi geri getir. Şartnameyi senin onayınla güncelle: ek özellikler [K] olarak, `.mee` uzantısı. Yeni bir `YOL_HARITASI.md` yaz: mevcut durum + kalan işler.
2. **Güvenlik ve veri:** Ç1 (parola), Ç3 (pano), Ç2 (sıra). Hepsi Opus işi, her biri ayrı commit ve test.
3. **Güncelleme:** `tauri-plugin-updater`'a geçiş + 1.3.1 kullanıcıları için köprü sürümü (G1–G4). İlk iş olarak 1.3.1'in elle kurulumu.
4. **Arayüz düzeltmeleri** A1–A6. Sonnet alt ajanı, Opus denetimi.
5. **Büyük dosyaları böl** (Ç6). Davranış değişmeden, testlerle.
6. **Gerçek ortam kabul testi** (senin elle yapacakların): Notepad, Word, Outlook, Chrome, kurum yazılımı.
