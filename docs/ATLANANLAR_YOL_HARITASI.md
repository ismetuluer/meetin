# Meetin — Atlanan ve Ertelenen Özellikler Yol Haritası

Bu belge, şartname ve ilk yol haritası incelemesinde henüz tam sürümüne ulaşmamış veya sadeleştirilmiş özelliklerin sonraki sürümlere yönelik tamamlanma planını içerir.

---

## Genel Bakış

`v1.1.0` itibarıyla temel metin genişletme motoru, Türkçe (`tr-TR`) harf uyumu, çoklu klasör + sürükle-bırak, `Shift+F3` büyük/küçük harf döngüsü, pano geçmişi, maskot etkileşimleri, tema motoru ve NSIS + Portable paketleme tamamlanmıştır. Aşağıdaki 5 başlık **Faz A → Faz E** sırasıyla hedeflenmektedir:

| Faz | Konu | Öncelik | Hedef Sürüm |
| --- | --- | --- | --- |
| **Faz A** | Windows 11 Mica / Acrylic Pencere Saydamlığı | Orta | `v1.2.0` |
| **Faz B** | `.mee` Zengin Medya ZIP Arşivi (Görseller / Ekler) | Orta | `v1.2.0` |
| **Faz C** | Tam WYSIWYG Düzenleyici (`@tiptap/react`) & Çoklu Dil (`en.json`) | Orta | `v1.3.0` |
| **Faz D** | Kod İmzalı Sessiz Öz-Güncelleme (`tauri-plugin-updater`) | Yüksek | `v1.3.0` |
| **Faz E** | 24 Saatlik Kararlılık, UIA / VM Otomatik Kabul Testleri | Yüksek | `v1.4.0` |

---

## Faz A — Windows 11 Mica / Acrylic Desteği
- **Mevcut Durum:** Pencereler CSS `backdrop-filter: blur(...)` ve tema arka plan tonlarını kullanıyor; DWM Mica/Acrylic çekirdeğe bağlı değil.
- **Yapılacaklar:**
  1. `window-vibrancy` crate'ini çevrimdışı bağımlılık önbelleğine ekle.
  2. `main` penceresinde Windows 11 (`build >= 22000`) için `apply_mica`, arama/ekleme/form adalarında `apply_acrylic` uygula; Windows 10'da düz Fluent yüzeyine otomatik düş.
  3. **Kabul Ölçütü:** Açık ve koyu Windows temalarında pencere arkası sistem duvar kâğıdına göre yumuşak bulanıklık gösterir; düşük GPU'lu cihazlarda kare hızı düşmez.

## Faz B — `.mee` Paket Biçimi (Zengin Medya Destekli ZIP)
- **Mevcut Durum:** `.mee` uzantısı JSON yapısında ifadeleri, HTML biçimlendirmesini ve klasörleri dışa/içe aktarır (`.meetin` geriye dönük olarak da açılır).
- **Yapılacaklar:**
  1. İfade içeriğine gömülü görseller (`assets/`) ve sürüm metadatasını tek bir `.mee` (ZIP konteyneri) içine paketle; düz JSON `.mee` dosyalarını da okumaya devam et.
  2. İçe aktarma sırasında imza/şema doğrulaması yap ve bozuk ZIP durumunda anlaşılır Türkçe hata ver.
  3. **Kabul Ölçütü:** Görsel içeren zengin bir kütüphane `.mee` olarak dışa aktarılıp başka bir bilgisayara kayıpsız aktarılabilir.

## Faz C — Tam WYSIWYG Düzenleyici (`@tiptap/react`) ve İngilizce (`en.json`)
- **Mevcut Durum:** `B / I / U / Bağlantı` düğmeleri `content_html` üretiyor; i18n altyapısı çoklu dile hazır ancak yalnızca `tr.json` dolu.
- **Yapılacaklar:**
  1. `@tiptap/react` + StarterKit ile canlı biçimli (kalın, italik, altı çizili, bağlantı, liste) WYSIWYG alanını entegre et; düz metin ↔ HTML dönüşümünü otomatik tut.
  2. `src/i18n/en.json` çeviri dosyasını tamamla ve **Ayarlar › Dil** sekmesine canlı `Türkçe / English` seçici ekle.
  3. **Kabul Ölçütü:** Kullanıcı görsel editörde biçim verdiği metni Word/Outlook'a birebir yapıştırabilir; arayüz dili yeniden başlatma gerektirmeden Türkçe/İngilizce değişir.

## Faz D — İmzalı Sessiz Güncelleme (`tauri-plugin-updater`) ve Kod İmzalama
- **Mevcut Durum:** GitHub Releases `latest.json` üzerinden sürüm kontrolü yapılıyor ve indirme bağlantısı açılıyor; portable öz-güncelleme hazırlayıcısı hazır.
- **Yapılacaklar:**
  1. Tauri güncelleme imzalama anahtar çifti üret (`tauri signer generate`) ve `.github/workflows/release.yml` içine `TAURI_SIGNING_PRIVATE_KEY` sırrını bağla.
  2. `tauri-plugin-updater` ile kurulumlu sürümde tek tıkla arka planda indirip yeniden başlatan akışı bağla.
  3. İsteğe bağlı Windows Authenticode (SmartScreen azaltma için EV veya bulut kod imzalama sertifikası) ekle.
  4. **Kabul Ölçütü:** Yeni etiket yayımlandığında mevcut kullanıcı tek tıkla doğrulamalı güncelleme alır.

## Faz E — Otomatik VM / 24 Saatlik Kararlılık ve Kabul Testleri
- **Mevcut Durum:** Tüm Rust birim/entegrasyon testleri (`53` test) ve Vitest ön yüz testleri (`20+` test) her derlemede çalışıyor.
- **Yapılacaklar:**
  1. Gerçek Windows sandbox/VM üzerinde Word, Excel, Chrome, Slack ve Notepad hedeflerine UIA + kanca UAT otomasyonu ekle.
  2. 24 saatlik bellek sızıntısı ve CPU yükü (%1 altı boşta) izleme testini CI raporuna ekle.
  3. **Kabul Ölçütü:** 24 saat sürekli açık kullanımda bellek artışı < 15 MB ve 0 çökme.
