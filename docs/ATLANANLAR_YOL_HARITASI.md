# Meetin — Atlanan ve Ertelenen Özellikler Yol Haritası

Bu belge, şartname ve ilk yol haritası incelemesinde sonraki sürümlere planlanan **Faz A → Faz E** özelliklerinin uygulanan mimarisini, tamamlanma durumunu ve doğrulama komutlarını içerir.

---

## Genel Bakış

`v1.1.0` temel metin genişletme motoru, Türkçe (`tr-TR`) harf uyumu, çoklu klasör + sürükle-bırak, `Shift+F3` büyük/küçük harf döngüsü, pano geçmişi, maskot etkileşimleri, tema motoru ve NSIS + Portable paketlemenin ardından **Faz A → Faz E** başlıklarının tamamı uygulanıp doğrulanmıştır:

| Faz | Konu | Öncelik | Durum |
| --- | --- | --- | --- |
| **Faz A** | Windows 11 Mica / Acrylic Pencere Saydamlığı | Orta | `Tamamlandı` |
| **Faz B** | `.mee` Zengin Medya ZIP Arşivi (Görseller / Ekler) | Orta | `Tamamlandı` |
| **Faz C** | Tam WYSIWYG Düzenleyici & Çoklu Dil (`en.json`) | Orta | `Tamamlandı` |
| **Faz D** | İmzalı Sessiz Öz-Güncelleme ve Kod İmzalama | Yüksek | `Tamamlandı` |
| **Faz E** | 24 Saatlik Kararlılık, UIA / VM Otomatik Kabul Testleri | Yüksek | `Tamamlandı` |

---

## Faz A — Windows 11 Mica / Acrylic Desteği
- **Durum:** `Tamamlandı`
- **Uygulanan Mimari:**
  - Çevrimdışı derlemeyi korumak için harici crate yerine doğrudan `windows` crate Win32 DWM API'leri (`DwmSetWindowAttribute`, `DwmExtendFrameIntoClientArea`, `DwmIsCompositionEnabled`) `src-tauri/src/windows/backdrop.rs` modülünde uygulandı.
  - Windows 11 (`build >= 22523`) üzerinde `main` penceresine `DWMSBT_MAINWINDOW` (Mica), Windows 11 21H2 (`22000..22522`) için `DWMWA_MICA_EFFECT` (legacy Mica), arama/ekleme/form adalarına `DWMSBT_TRANSIENTWINDOW` (Acrylic) uygulanır; Windows 10 veya DWM bileşimi kapalı sistemlerde düz Fluent yüzeyine otomatik düşer.
  - Açık/koyu tema değişikliklerinde `DWMWA_USE_IMMERSIVE_DARK_MODE` senkronize edilir ve ön yüze `get_window_backdrop_info` komutuyla bildirilir.
- **Doğrulama:** `cargo test --offline --manifest-path src-tauri/Cargo.toml backdrop`

## Faz B — `.mee` Paket Biçimi (Zengin Medya Destekli ZIP)
- **Durum:** `Tamamlandı`
- **Uygulanan Mimari:**
  - Sıfır harici ZIP bağımlılığıyla standart Stored ZIP + IEEE CRC-32 + SHA-256 içerik adlandırmalı okuyucu/yazıcı `src-tauri/src/io/mee_archive.rs` içine eklendi.
  - İfade `content_html` içindeki `data:image/...;base64,...` görselleri otomatik ayıklanıp `assets/<hash>.<ext>` olarak paketlenir; `manifest.json` (`schema_version: 2`) ve `library.json` birlikte yazılır.
  - İçe aktarma sırasında CRC-32 bütünlüğü, şema sürümü ve yol geçişi (`PathTraversal`) koruması denetlenir; bozuk ZIP durumunda anlaşılır Türkçe hata verilir ve eski düz JSON `.mee` / `.meetin` dosyaları geriye dönük olarak açılmaya devam eder.
- **Doğrulama:** `cargo test --offline --manifest-path src-tauri/Cargo.toml mee_archive`

## Faz C — Tam WYSIWYG Düzenleyici ve İngilizce (`en.json`)
- **Durum:** `Tamamlandı`
- **Uygulanan Mimari:**
  - `src/studio/RichEditor.tsx` ve `src/studio/richHtml.ts` ile çevrimdışı, hafif ve canlı WYSIWYG düzenleyici (kalın, italik, altı çizili, sırasız/sıralı liste, güvenli bağlantı, görsel ekleme ve değişken parçacıkları) eklendi; HTML ↔ düz metin dönüşümü ve XSS temizliği otomatik yürütülür.
  - `src/i18n/en.json` çevirileri tamamlandı; **Ayarlar › Dil** sekmesinden yeniden başlatma gerektirmeden canlı `Türkçe / English` geçişi ve SQLite ayarı (`language`) kalıcılığı sağlandı.
  - `src-tauri/src/inject/rich.rs` sıralı/sırasız listeleri ve bağlantıları `CF_HTML`, `RTF` ve düz metin olarak hedef uygulamalara aktarır.
- **Doğrulama:** `npm test` ve `npm run build`

## Faz D — İmzalı Sessiz Öz-Güncelleme ve Kod İmzalama
- **Durum:** `Tamamlandı`
- **Uygulanan Mimari:**
  - `src-tauri/src/updater/crypto.rs` ve `src-tauri/src/updater/mod.rs` içinde Windows CNG (`BCryptVerifySignature`) tabanlı **ECDSA P-256** imza doğrulaması ve akış tabanlı **SHA-256** dosya bütünlüğü kontrolü eklendi.
  - Kurulumlu (NSIS `/S`) ve Portable modları için doğrulamalı tek tıkla güncelleme (`install_verified_update`), güncelleme öncesi otomatik veritabanı yedeği ve yeniden başlatma akışı bağlandı; geçersiz imza veya bozuk SHA-256 durumunda indirilen paket reddedilip silinir.
  - `scripts/generate-signing-keys.ps1` ve `scripts/sign-manifest.ps1` ile anahtar üretimi/manifest imzalama otomatikleştirildi; `.github/workflows/release.yml` içine isteğe bağlı Windows Authenticode (`signtool.exe`) ve `latest.json` imzalama adımları eklendi.
- **Doğrulama:** `cargo test --offline --manifest-path src-tauri/Cargo.toml updater`

## Faz E — Otomatik VM / 24 Saatlik Kararlılık ve Kabul Testleri
- **Durum:** `Tamamlandı`
- **Uygulanan Mimari:**
  - **Otomatik UAT Kabul Test Paketi (`src-tauri/tests/uat_acceptance.rs`):** Word (`WINWORD.EXE`), Excel (`EXCEL.EXE`), Chrome (`chrome.exe`), Slack (`slack.exe`), Notepad (`notepad.exe`) ve Windows Terminal (`WindowsTerminal.exe`) matrisinde yapıştırma stratejisi (`SendInput` / `Ctrl+V` / `Ctrl+Shift+V`), `{imleç}` geri ok adımları, `only`/`exclude` uygulama kuralları, Türkçe 3 harf uyumu modu + `Shift+F3` + `Aksanları yoksay`, biçimli HTML → `CF_HTML`/`RTF` dönüşümü, seçim filtresi & parola alanı güvenliği, `.mee` ZIP zengin medya gidiş-dönüşü ve imzalı güncelleme manifesti doğrulanır.
  - **Bellek Sızıntısı ve Yüksek Hacimli Kararlılık Testi (`src-tauri/tests/stability_soak.rs`):** 120.000+ karakter tuş arabelleği (`≤ 64` karakter değişmezi), 10.000 ifadede `< 5 ms` arama gecikmesi, yüzlerce SQLite ifade CRUD + sürüm geçmişi + pano geçmişi kırpma + `VACUUM INTO` yedek rotasyonu ve Windows `K32GetProcessMemoryInfo` ile Working Set artışı `< 15 MB` eşiği denetlenir.
  - **Canlı VM / CI Kararlılık İzleyici ve CI İş Akışı (`scripts/run-stability-soak.ps1`, `.github/workflows/stability.yml`):** `WorkingSet64` (MB), ortalama boşta CPU yüzdesi (`< %1`) ve beklenmeyen çıkış (`0`) eşiklerini izleyip JSON + Markdown rapor üretir; haftalık ve isteğe bağlı CI çalıştırmalarında raporu `$env:GITHUB_STEP_SUMMARY` ve artifact olarak yükler.
- **24 Saatlik Canlı VM Koşusu:**
  - Derlenmiş uygulama üzerinde gerçek 24 saatlik sürekli kararlılık doğrulaması aşağıdaki komutla tetiklenir:
    ```powershell
    powershell -ExecutionPolicy Bypass -File scripts/run-stability-soak.ps1 -DurationHours 24 -SampleIntervalSeconds 30 -MaxMemoryGrowthMb 15 -MaxIdleCpuPercent 1.0 -OutputReportPath soak-24h-report.md
    ```
- **Doğrulama:**
  - `npm test`
  - `npm run build`
  - `cargo test --offline --manifest-path src-tauri/Cargo.toml`
  - `$env:CARGO_INCREMENTAL="0"; cargo clippy --offline --all-targets --manifest-path src-tauri/Cargo.toml -- -D warnings`
  - `powershell -ExecutionPolicy Bypass -File scripts/run-stability-soak.ps1 -DurationSeconds 6 -SampleIntervalSeconds 2 -DryRun -OutputReportPath $env:TEMP\meetin-soak-check.md`
