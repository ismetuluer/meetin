# Meetin · Akıllı Türkçe Metin Genişletici (`v1.1.0`)

Meetin, Windows için tasarlanmış yerel, hızlı ve Türkçe (`tr-TR`) uyumlu akıllı metin genişletici ve üretkenlik maskotudur.

## Özellikler

- **Türkçe Harf Uyumu (`tr-TR`):** `i ↔ İ`, `ı ↔ I` ayrımını bozmadan *Yazışa uy*, *Aynen getir* ve *Birebir eşleşme* modlarıyla genişletme ve *Aksanları yoksay* seçeneği.
- **Evrensel `Shift+F3` Harf Döngüsü:** Herhangi bir düzenlenebilir Windows programında seçili metni Word tarzı `küçük → İlk harf büyük → TÜMÜ BÜYÜK` döngüsüyle tek kısayolla dönüştürme.
- **Zengin İçerik ve Değişkenler:** Word/Outlook için `CF_HTML` + Türkçe Unicode kaçışlı `RTF`, `{tarih}`, `{tarih:±g/h/a/y}`, `{saat}`, `{pano}`, `{imleç}`, `{alan:…}` form adası ve `{ifade:…}`.
- **Maskot ve Dinamik Ada:** Masaüstünde serbestçe sürüklenip fırlatılabilen koyun maskotu (gerçek mee, fırlatma ve duvar çarpma ses efektleriyle), yazarken önek öneri adası (`Tab`), seçili metinde `+ Ekle` balonu (doğrudan yeni klasör ekleme) ve basketbol mini oyunu.
- **Ana Arayüz (Fluent 2):** Daraltılabilir sol kenar çubuğu, sürükle-bırak ile klasöre taşıma (`Ctrl+bırak` ile çoklu klasöre kopyalama), tekil kısaltma koruması, otomatik kaydetme, içerik→açıklama otomatik eşitleme, açık uygulamaları otomatik algılayan kurallar, Pano Geçmişi (hatırlama/limit ve büyük/küçük harf kopyalama seçenekleri), Çöp Kutusu, sürüm geçmişi, `Ctrl+K` Komut Paleti ve `.mee` / CSV / `.ahk` / `.txt` içe/dışa aktarma.
- **Tema ve OKLCH Palet Motoru:** Windows açık/koyu modu ve vurgu rengini canlı izler; WCAG ≥ 4,5:1 karşıtlık güvenceli 3 düzeyli renk özelleştirme ve `.mee-tema` paylaşımı sunar.
- **Güvenilir Yedekleme (`%APPDATA%\Meetin`):** Tüm veriler kullanıcı `AppData\Roaming\Meetin\` klasöründe tutulur; açılış ve kapanışta otomatik SQLite `VACUUM INTO` yedeği (30 kopya rotasyonu) alınır.

## Varsayılan Kısayollar

| Kısayol | İşlev |
| --- | --- |
| `Ctrl + Alt + F` | Arama adasını aç |
| `Ctrl + Alt + E` | Seçili metinden hızlı ekle |
| `Ctrl + Alt + S` | Ana pencereyi aç |
| `Ctrl + Alt + P` | Motoru duraklat / sürdür |
| `Ctrl + Alt + Space` | Maskotu göster / gizle |
| `Ctrl + K` | Ana pencerede Komut Paleti |
| `Shift + F3` | Seçili metnin büyük/küçük harf biçimini değiştir |

## Kurulum ve Taşınabilir (Portable) Kullanım

- **Kurulumlu (NSIS):** `Meetin_1.1.0_x64-setup.exe` kullanıcı düzeyinde kurulur.
- **Taşınabilir (Portable):** `Meetin_1.1.0_portable.zip` kurulum gerektirmeden doğrudan çalışır ve kullanıcı verilerini varsayılan olarak `%APPDATA%\Meetin\` altında saklar (yerel `.\data\` klasörüne almak için EXE yanına boş `local-data.flag` dosyası eklenebilir).
- **Geliştirme başlatma:** Projeden doğrudan çalıştırmak için [Start.bat](Start.bat) dosyasını kullanabilirsiniz.
- **Portable paket oluşturma:** `scripts\package-portable.ps1` betiği `dist-packages\Meetin_1.1.0_portable.zip` üretir.
- **Atlanan Özellikler Yol Haritası:** Gelecek sürümler için planlanan özellikler [docs/ATLANANLAR_YOL_HARITASI.md](docs/ATLANANLAR_YOL_HARITASI.md) dosyasında yer alır.
- **GitHub Otomatik Sürüm ve Güncelleme:** `.github/workflows/release.yml`, `v*` etiketi itildiğinde NSIS kurulum (`Meetin_<sürüm>_x64-setup.exe`), Portable ZIP (`Meetin_<sürüm>_portable.zip`) ve `latest.json` dosyalarını [ismetuluer/meetin Releases](https://github.com/ismetuluer/meetin/releases/latest) altında otomatik yayımlar; Meetin varsayılan olarak `https://github.com/ismetuluer/meetin/releases/latest/download/latest.json` üzerinden güncellemeleri kontrol eder.

## Windows SmartScreen Uyarısı (§9)

Meetin açık kaynaklı ve kod imzalama sertifikası olmadan dağıtıldığı için Windows Defender SmartScreen ilk çalıştırmada *"Windows kişisel bilgisayarınızı korudu"* uyarısı gösterebilir. Devam etmek için **Ek bilgi** → **Yine de çalıştır** adımını seçebilirsiniz.
