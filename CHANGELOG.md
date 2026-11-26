# Değişiklik Günlüğü (CHANGELOG)

## [1.3.0] - 2026-11-26 (Mica/Acrylic · .mee ZIP Medya · WYSIWYG & İngilizce · İmzalı Güncelleme · Yardım Merkezi · UAT/Kararlılık)

### Eklenenler ve İyileştirmeler
- **Faz A · Windows 11 Mica / Acrylic saydamlığı:** `Win32_Graphics_Dwm` ile Windows 11 ana penceresinde gerçek **Mica**, arama/ekleme/form adalarında **Acrylic** arka plan, kompakt maskot ve Windows 10'da güvenli Fluent geri düşümü + `DWMWA_USE_IMMERSIVE_DARK_MODE` canlı senkronu eklendi.
- **Faz B · `.mee` Zengin Medya ZIP arşivi:** Standart ZIP (`manifest.json`, `library.json`, SHA-256 tekilleştirilmiş `assets/<hash>.<ext>`) dışa/içe aktarma desteği getirildi; eski düz JSON `.mee`/`.meetin` dosyalarıyla geriye dönük tam uyumluluk, bozuk arşiv/CRC-32/path-traversal doğrulaması ve düzenleyiciye görsel ekleme/önizleme eklendi.
- **Faz C · Tam WYSIWYG düzenleyici ve İngilizce (`en.json`):** Canlı `RichEditor` (Kalın, İtalik, Altı çizili, Madde/Numaralı liste, URL doğrulamalı Bağlantı, Biçimi temizle, Görsel ekle, Değişken ekle + XSS beyaz liste temizleyici, CF_HTML/RTF `ol`/`a` desteği) ve **Ayarlar › Dil** altından yeniden başlatmadan tüm pencerelerde anında geçiş yapan **Türkçe / English** desteği eklendi.
- **Faz D · Kriptografik İmzalı Sessiz Güncelleme:** Windows CNG ECDSA P-256 + akış SHA-256 doğrulamalı `install_verified_update` komutu (`Meetin` NSIS `/S` sessiz kurulum/yeniden başlatma ve Portable `Meetin.old.exe` yükseltme), **Ayarlar › Güncelleme** içinde imza rozeti + tek tık **"İndir ve Güncelle"**, isteğe bağlı CI imzalama adımları ve imzalama betikleri (`scripts/generate-signing-keys.ps1`, `scripts/sign-manifest.ps1`, `docs/GUNCELLEME_IMZALAMA.md`) eklendi.
- **Faz E · UAT & Kararlılık Altyapısı:** 6 hedef uygulama (`WINWORD.EXE`, `EXCEL.EXE`, `chrome.exe`, `slack.exe`, `notepad.exe`, `WindowsTerminal.exe`) için otomatik kabul test paketi (`uat_acceptance.rs`), 120k+ tuş / SQLite / `K32GetProcessMemoryInfo` bellek sızıntısı soak testleri (`stability_soak.rs`), `scripts/run-stability-soak.ps1` canlı 24 saat izleyici ve haftalık CI iş akışı (`.github/workflows/stability.yml`) eklendi; UAT sırasında yakalanan ASCII işlem adı Türkçe-`I` eşleşme farkı düzeltildi.
- **Uygulama içi Yardım Merkezi (`F1`) ve Gizlilik Temizliği:** Aranabilir **Yardım Merkezi** (`HelpView`) ve [docs/YARDIM.md](docs/YARDIM.md) eklendi; tüm arayüz, test, doküman ve gerçek uygulama ekran görüntüleri genel kullanıma uygun olarak temizlendi.

## [1.2.0] - 2026-11-26 (Ses Ayarları · Shift+F3 Döngüsü · Toplu Seçim · Sola Kaydır Sil · Klasör Renkleri)

### Eklenenler ve İyileştirmeler
- **Ayrı maskot ses kontrolleri & sağ-tık sessize alma:** Ana ses anahtarının yanında *Mee sesi*, *Fırlatma sesi* ve *Duvar çarpma sesi* ayrı ayrı açılıp kapatılabilir; rahatsız edici çığlık sesi kaldırıldı, otomatik *mee* sıklığı azaltıldı ve maskota sağ tıklayınca doğrudan **Sessize Al / Sesi Aç** eklendi.
- **Ardı ardına `Shift+F3` büyük/küçük harf döngüsü:** `Shift+F3` yapıştırma sonrası `Shift+Left` ile metni yeniden seçili bırakır ve fiziksel Shift tuşlarını temizler; böylece seçim bozulmadan art arda `küçük → İlk büyük → TÜMÜ BÜYÜK` geçişi yapılır.
- **Toplu seçme, sürükle-sil, sola kaydırıp silme & sağ-tık bağlam menüsü:** Listede onay kutusu / `Ctrl+Tık` / `Shift+Tık` ile toplu seçme; toplu veya tekli ifadeleri Çöp Kutusuna sürükleyerek ya da satırı sola kaydırıp kırmızı **Sil** ile silme; ifade ve klasörlerde özel sağ-tık bağlam menüsü (`Düzenle`, `Klasöre taşı/kopyala`, `Dışa aktar`, `Sil`, `Yeniden adlandır`, `Renk ata`).
- **Klasör renkleri, liste dışa aktarma & klasör etiket eşitleme:** Klasörlere 8 renk atanabilir; görüntülenen listeler veya seçili ifadeler doğrudan `.mee` / `CSV` / `.ahk` olarak dışa aktarılabilir; yeni/seçili klasör adı otomatik olarak etiketlere eklenir.
- **Yazı boyutu ayarı & sade güncellemeler:** *Ayarlar › Görünüm* altına **Yazı boyutu** (*Küçük / Normal / Büyük / Çok Büyük*) eklendi; varsayılan harf uyumu *Aynen getir* yapıldı, güncelleme URL'si arayüzden gizlendi, kullanıcı adı yer tutucusu sadeleştirildi ve `Harf uyumu` / `Büyük/Küçük Harf Uyumu` etiketleri güncellendi.
- **GitHub görsel galerisi:** `README.md` içine maskot ve gerçek uygulama ekran görüntüleri eklendi.

## [1.1.0] - 2026-11-26 (Çoklu Klasör · Otomatik Kaydetme · Shift+F3 · Pano & Ses İyileştirmeleri)

### Eklenenler ve İyileştirmeler
- **Evrensel `Shift+F3` büyük/küçük harf döngüsü:** Hangi programda olunursa olunsun seçili düzenlenebilir metinde `Shift+F3` ile `küçük → İlk harf büyük → TÜMÜ BÜYÜK` geçişi (`Ayarlar › Harf uyumu` üzerinden açılıp kapatılabilir).
- **Çoklu klasör üyeliği, sürükle-bırak & tekil kısaltma:** İfadeler sürükle-bırak ile klasörlere taşınabilir; `Ctrl` basılı tutularak bırakılırsa silinmeden hedef klasöre de eklenir (birden fazla klasörde yer alır). Aynı kısaltmanın birden fazla kez oluşturulması veritabanı düzeyinde engellenir.
- **Otomatik kaydetme ve içerik→açıklama eşitleme:** İfade ekleme/düzenleme taslakları 650 ms sonra otomatik kaydedilir; içeriğe yazılan ilk satır, açıklama alanına elle müdahale edilmediği sürece açıklamaya otomatik yansır.
- **Daraltılabilir sol menü, kompakt harf uyumu ve bağlantı simgesi:** Sol menü daraltılıp genişletilebilir ve son görünüm ayarları hatırlanır; içerik çubuğundaki `Bağ` metni bağlantı simgesine, `Harf uyumu` ise açılır menülü kompakt `Aa`/`A=`/`AA` simgesine dönüştürüldü.
- **Açık uygulama algılama & hızlı klasör oluşturma:** Kurallar ve Dışlanan Programlar ekranlarında o an çalışan pencereli `.exe` uygulamaları otomatik listelenir; Hızlı İfade Ekleme adasında klasör listesi altından tek adımda yeni klasör oluşturulabilir.
- **Pano geçmişi ayarları ve harf dönüştürerek kopyalama:** `Genel` ayarlarına `Panoyu hatırla` ve `Saklanan son kopya sayısı` (varsayılan `10`) eklendi; Pano Geçmişi kartlarına `Kopyala`, `TÜMÜ BÜYÜK`, `tümü küçük`, `İlk Harf Büyük` düğmeleri eklendi.
- **Otomatik kısaltma öneri kuralı:** Ayarlardan `İlk harfler`, `İlk kelime`, `Son kelime` veya `Etiket` seçilebilir.
- **Gerçek fırlatma/duvar çarpma sesleri ve daha belirgin ikonlar:** `sounds/` klasöründeki whoosh, scream ve 3 darbe/çarpma MP3 efekti bağlandı; `icon.ico` ve tepsi ikonları tüm görev çubuğu boyutları için büyütülüp keskinleştirildi.
- **Sağ-tık web menüsü kapatıldı, AppData varsayılanı ve `.mee` uzantısı:** Ana pencerede WebView2 sağ-tık menüsü engellendi; veriler taşınabilir sürüm dahil `%APPDATA%\Meetin` altında saklanır, varsayılan dışlanan exe listesi ve `Uzman arayüzü` metinleri kaldırıldı, `.meetin` yerine `.mee` uzantısına geçildi.
- **Atlanan özellikler yol haritası:** `docs/ATLANANLAR_YOL_HARITASI.md` oluşturuldu.

## [1.0.1] - 2026-11-26 (Başlangıç State Düzeltmesi · Gerçek Sesler · Aksan Katlama · GitHub Release)

### Düzeltmeler ve İyileştirmeler
- **`state not managed` başlangıç hatası giderildi:** `HookController` ve `AppState` artık pencere açılmadan önce `Builder::manage()` üzerinde kaydediliyor; açılışta `list_phrases` yarış durumu oluşmaz.
- **Gerçek maskot sesleri:** Kullanıcı tarafından eklenen `baaa.mp3` koyun sesi olarak bağlandı; sürükleyip fırlatma anında Web Audio hava vınlaması (whoosh) efekti eklendi (`Maskot` ayarlarından test edilebilir).
- **Kısayol yakalayıcı ve ifade kısayolları:** Ayarlar ve ifade düzenleyicide tuşa basarak `Ctrl+Alt+...` biçiminde kısayol kaydetme desteği, ifadeye özel atanmış kısayolların genel kısayol olarak tetiklenmesi.
- **Aksanları yoksay & AppData kısayolu:** Türkçe aksanlı harfleri (`ç/ğ/ı/ö/ş/ü/â/î/û`) aksansız karşılıklarıyla eşitleyen `Aksanları yoksay` ayarı ve `Genel` altında kullanıcı AppData klasörünü açan `Klasörü Aç` düğmesi.
- **Arayüz temizliği ve ipuçları (tooltips):** Örnek metinler sadeleştirildi, tüm düğme ve alanlara açıklayıcı Türkçe ipuçları eklendi.
- **GitHub Releases ve `https://` güncelleme:** `.github/workflows/release.yml` sürüm iş akışı ve `updater.rs` içinde `https://` `latest.json` okuyucusu eklendi.

## [1.0.0] - 2026-11-26 (M6.10 · Kararlı Sürüm)

### Eklenenler ve İyileştirmeler
- **Salt okunur portable dayanıklılığı:** `Storage::open` salt okunur ortamlarda (ör. salt okunur USB) mevcut `meetin.db` dosyasını otomatik salt okunur kipte (`SQLITE_OPEN_READ_ONLY`) açarak sorunsuz okuma ve genişletme sağlar.
- **Sürüm belgeleri ve dağıtım paketi:** `README.md` (SmartScreen notu ve kısayol/kurulum rehberi dahil) oluşturuldu; release çalıştırılabilir dosya ve `Meetin_1.0.0_portable.zip` paketi doğrulandı.

## [0.5.9] - 2026-11-26 (Öneri Adası M6.7 · Başlangıç M6.8 · Paketleme & Güncelleyici M6.9)

### Eklenenler
- **Yazarken Öneri Adası (M6.7):** 2+ harfte önek eşleşmesiyle en fazla 3 kısaltma hapı gösteren odak çalmayan öneri şeridi, `Tab` veya tıkla tamamlama; parola kutuları ve dışlanan programlarda otomatik gizli.
- **Windows Başlangıcı ve Yönetici Görevi (M6.8):** `HKCU\...\Run` üzerinden Windows ile başlatma, UAC istemeden `schtasks` (`/RL HIGHEST`) ile yönetici görevi oluşturma ve canlı yükseltme rozeti.
- **Paketleme ve Güncelleyici (M6.9):** Türkçe NSIS kurulum + WebView2 bootstrapper yapılandırması, `scripts/package-portable.ps1` portable ZIP betiği, `latest.json` ayrıştırıcısı, maskot güncelleme noktası ve `Meetin.old.exe` öz-güncelleme hazırlayıcısı (`data\` korunur).

## [0.5.6] - 2026-11-26 (Ayarlar M6.4 · Yedek Geri Yükleme M6.5 · İlk Açılış M6.6)

### Eklenenler
- **Canlı Ayar Sayfaları (M6.4):** Tetikleme tuş grupları (Boşluk / Enter / Tab / Noktalama), Harf uyumu örnek tablosu, Maskot boyut (`small`/`normal`/`large`) ve Mee sesi tercihleri, çakışma kontrollü Kısayol düzenleyici ve Dışlanan Programlar yönetimi veritabanı ve klavye kancasına canlı bağlandı.
- **Yedekleme ve Emniyetli Geri Yükleme (M6.5):** `create_backup_now`, `list_backups` ve `restore_backup` ile `backups\` içindeki SQLite yedeklerini listeleme ve mevcut veritabanının `pre-restore-*` emniyet yedeğini alarak tek tıkla geri yükleme.
- **İlk Açılış Tanıtımı (M6.6):** Uyuyan koyundan (`sleep` → `hop`) uyanan 3 adımlı tanıtım sihirbazı, isim sorma ve isteğe bağlı `mrb` başlangıç ifadesi.

## [0.5.3] - 2026-11-26 (Windows 11 Görsel Tasarım + Tema Sistemi M6.1–M6.3)

### Eklenenler ve İyileştirmeler
- **Windows 11 / Fluent 2 Uzman Arayüzü:** Kesintisiz 3 sütunlu yerleşim (kenar çubuğunda Son Kullanılanlar, Pano Geçmişi, Klasörler, Çöp Kutusu, Haftalık Kazanç kartı ve Ayarlar; orta sütunda kompakt kart listesi; sağda inline başlık, etiket çipleri, birleşik `İçerik` kartı ve canlı değişken hapları, `Kısaltma` / `Kısayol tuşu` yan yana alanı, `Kurallar` çipleri ve kullanım kartları).
- **OKLCH Palet Üretici (M6.1):** Sıfır bağımlılıkla 11 basamaklı vurgu ölçeği, vurgu tonundan türetilen nötrler, açık/koyu tema ve tüm çiftlerde WCAG ≥ 4,5:1 karşıtlık güvencesi.
- **Windows Tema Senkronizasyonu (M6.2):** Registry (`AppsUseLightTheme` + `DWM\AccentColor`) üzerinden Windows açık/koyu modu ve vurgu rengini 2 saniyelik izleyiciyle tüm pencerelere (`main`, `mascot`, `bubble`, `hoop`) canlı yansıtma, `forced-colors` yüksek karşıtlık desteği.
- **Ayarlar & Görünüm Sayfası (M6.3):** Üç düzeyli renk özelleştirme (Sistemle uyumlu / Ana renk + Ton kaydırıcı / Rol bazlı tek tek düzenleme ve WCAG karşıtlık rozetleri), Canlı Önizleme paneli, Ada yüzeyi ve saydamlık ayarları ile `.meetin-tema` içe/dışa aktarma.

## [0.5.0] - 2026-11-26 (M5 · Uzman Arayüzü)

### Eklenenler
- **Üç panelli Stüdyo arayüzü:** Sol kenar çubuğu (Tüm İfadeler, Son Kullanılanlar, Pano Geçmişi, İstatistikler, Çöp Kutusu ve Klasörler), orta arama/sıralama/aksiyon paneli ve sağda `İçerik` / `Kurallar` / `Geçmiş` sekmeli düzenleyici.
- **Zengin içerik ve değişken çubuğu:** Düzenleyicide kalın/italik/altı çizili biçimlendirme düğmeleri, tek tıkla değişken ekleme (`{tarih}`, `{saat}`, `{pano}`, `{imleç}`, `{alan:…}`, `{ifade:…}`) ve ifade başına uygulama kuralı yönetimi.
- **Sürüm geçmişi ve Çöp Kutusu:** `list_phrase_versions` / `restore_phrase_version` ile tek tıkla eski sürüme dönüş; 30 günlük otomatik temizlik (`purge_expired_trash`) ve kalıcı silme/boşaltma.
- **Pano geçmişi izleyici:** Arka planda son 100 metin kopyasını kaynak uygulama bilgisiyle kaydeder (`ExcludeClipboardContentFromMonitorProcessing` ve `CanIncludeInClipboardHistory = 0` hassas kopyaları atlar) ve tek tıkla ifadeye dönüştürür.
- **Çok biçimli içe/dışa aktarma:** `.ahk` (AutoHotkey hotstring), `.txt` (boş satır blokları + tr-TR baş harf önerisi), Beeftext/Meetin JSON ve `.meetin` / CSV (UTF-8 BOM) / AHK dışa aktarımı.
- **Hızlı Toplu Ekle ve Komut Paleti (`Ctrl+K`):** Çok satırlı hızlı ifade girişi ve klavye odaklı komut/ifade arama paleti.

## [0.4.0] - 2026-11-26 (M4 · Zengin İçerik + Ada Kapatma)

### Eklenenler
- **Değişken çözümleyici (M4.1):** `{tarih}`, `{tarih:uzun}`, `{tarih:±g/h/a/y}` (artık yıl ve ay sonu uyumlu), `{saat}`, `{pano}`, `{imleç}`, `{alan:…}` ve döngü korumalı `{ifade:KISALTMA}`.
- **Biçimli metin yapıştırma (M4.2):** `CF_HTML` + Türkçe Unicode kaçışlı `RTF` + `CF_UNICODETEXT` ile Word/Outlook uyumlu biçimli yapıştırma, biçimsiz hedeflere düz metin.
- **`{imleç}` ve geri al (M4.3):** Genişletme sonrası gereken sayıda sol ok (`VK_LEFT`); genişletmeden hemen sonra `Backspace` ile tek adımda kısaltmaya geri dönüş.
- **Alan doldurma adası (M4.4):** `{alan:İsim}` ve `{alan:Seçenek|A,B}` alanları için kompakt form adası (`Tab` / `Enter` / `Esc`).
- **Program kuralları ve yapıştırma yöntemi (M4.5):** `get_phrase_app_rules` / `set_phrase_app_rules` DB API'si ve Windows Terminal (`wt.exe`, `windowsterminal.exe`) için `Ctrl+Shift+V` desteği.
- **Ada içi maskot kapatma:** Arama, Ekleme veya Alan Doldurma adası açıkken üstteki maskot simgesine tekrar tıklayınca ada penceresi kapanır.

## [0.3.1] - 2026-11-26

### İyileştirmeler
- **Sürüklerken Süper Koyun:** Maskot tutulup sürüklenirken de kırmızı pelerinini giyer ve çekilen yöne doğru toynağını uzatır.
- **Başparmak Onay tetikleri ve yazısız genişletme:** Yeni ifade eklendiğinde (`phrase-created`) ve ifade genişletildiğinde (`phrase-expanded`) yazı/hap göstermeden yalnızca Başparmak Onay (`thumbsup`) animasyonu oynatılır.
- **Geri silip yeniden yazma hafızası:** Eşleşmeyen noktalama/tetik tuşlarında arabellek sıfırlanmak yerine geçmişte tutulur; böylece yazım hatası veya noktalama girip Backspace ile sildikten sonra kısaltma tamamlandığında genişletme sorunsuz tetiklenir.

## [0.3.0] - 2026-11-26

### Eklenenler
- **Başparmak Onay (`thumbsup`) animasyonu:** 1. görselden uyarlanan iki yanda yukarı kalkık toynak-başparmak ve özgüvenli gülümseme tepkisi. CSV içe aktarma tamamlandığında (`file-imported`) ve basket atıldığında oynar.
- **Süper Koyun (`super`) uçuş modu:** 2. görselden uyarlanan dalgalanan kırmızı pelerin ve hız vektörü yönüne (`flightAngleDeg`) uzanan toynak animasyonu. Fırlatılınca otomatik giyer, kenarlardan sektikçe yönünü günceller.
- **Masaüstü Basketbol Mini Oyunu:** Maskot sağ tık menüsünden açılan şeffaf pota (`hoop` penceresi), yerçekimli parabol fiziği, file animasyonu, skor/seri sayacı ve basket sonrası yeni pota konumu.

## [0.2.6] - 2026-11-26

### Eklenenler
- **Her yönde 2D sallama:** `shakeDetector` artık yatay, dikey ve çapraz fark etmeksizin tüm yönlerdeki sallamayı algılayarak uzman ekranını açıp kapatır.
- **İvmeli fırlatma ve ekran köşelerinden sekme:** `throwPhysics` ve `get_mascot_monitor_bounds` ile maskotu sürükleyip fırlatabilirsiniz. Hız kademesine göre maskot sevinir (`hop`) veya meler (`baa`); yüksek hızda ekran kenar ve köşelerine çarpıp esnekçe seker, sürtünmeyle durur.

## [0.2.5] - 2026-11-26

### Eklenenler
- **Maskotu sallayarak uzman ekranını aç/kapa:** Koyun tutulup hızlıca sağa-sola sallandığında (`shakeDetector`) ayrıntılı uzman penceresi açılır; tekrar sallandığında gizlenir.

## [0.2.4] - 2026-11-26 (M3.1–M3.4 · İfade Ekleme Yolları)

### Eklenenler
- **Seçili metni okuma (M3.1):** `selection/` modülü UI Automation `IUIAutomationTextPattern` üzerinden seçili metni okur (parola kutularını reddeder); desteklemeyen uygulamalarda panoyu yedekleyip `Ctrl+C` ile okur ve panoyu eksiksiz geri yükler.
- **Seçim algılama ve `+ Ekle` balonu (M3.2):** Fare sürükleme / çift-üç tık seçimi sonrası ~300 ms bekleyip imlecin yanında odak çalmayan şeffaf `+ Ekle` balonu açılır; 3 sn içinde ya da yazma/tıklama devam edince sessizce gizlenir. `photoshop.exe` ve dışlanan programlarda gizlenir.
- **Ekleme adası ve üç ekleme yolu (M3.3 & M3.4):**
  - Seçim balonu, `Ctrl+Alt+E` kısayolu ve maskotu ≥ 450 ms basılı tutup sıkma (`open_add_from_squeeze`) ile Ekleme Adası açılır.
  - tr-TR baş harf kısaltma önerisi (`"Metin ile Pratik Yazım"` → `"MİPY"`), anlık çakışma uyarısı, klasör ve etiket alanları, `"Panodakini ifade yap"`, `"Ayrıntılı düzenle"` ve `Enter` ile kaydedince `eat` animasyonu + onay hapı.

## [0.1.8] - 2026-11-26

### Düzeltilenler
- **Maskot ve Arama Adası serbest taşıma:** `WS_EX_NOACTIVATE` ile çakışan `startDragging()` yerine `setPointerCapture` + `SetWindowPos(..., SWP_NOACTIVATE)` tabanlı akıcı sürükleme getirildi. Artık hem maskot hem de açılan Arama Adası üst çubuğundan serbestçe sürüklenebilir.
- **Tepkisellik ve ekran sınır koruması:** `pointercancel`/`lostpointercapture` temizliği ile maskotun takılı kalması giderildi; açılan ada ekran dışına taşmayacak şekilde monitör sınırlarına hizalanır ve kapanınca eski maskot konumuna döner.

## [0.1.7] - 2026-11-26

### Eklenenler ve Düzeltilenler
- **Maskot penceresi gerçek saydamlığı:** `html.is-mascot-window` kök kuralı ve WebView2 şeffaf arka plan rengi (`Color(0, 0, 0, 0)`) ile maskot arka planındaki opak kare kaldırıldı.
- **Evrensel `Esc` ve odak kaybı kapanışı:** Arama adası veya sağ tık menüsü açıkken odak nerede olursa olsun `Esc` ile ya da pencere odak kaybettiğinde otomatik olarak maskot durumuna dönülür.
- **Tek tıkla çalıştırma:** Proje köküne `Start.bat` eklendi.

## [0.1.6] - 2026-11-26 (M2.1–M2.6 · Maskot ve Yüzen Ada)

### Eklenenler
- **Maskot bileşeni ve durum makinesi (M2.2):** `MeetinSheep` (`docs/meetin-maskot.svg` ve `docs/meetin-maskot-animasyon.html` birebir Harman B SVG yapısı, katman adları ve 9 CSS animasyonu: `idle`, `graze`, `baa`, `stare`, `sleep`, `eat`, `hop`, `sad`, `squeeze`), göz bebeği takibi, "kendi hâlinde" modu, `prefers-reduced-motion`, `forced-colors`, gizliyken animasyon durdurma ve varsayılan kapalı Web Audio mee sesi.
- **Yüzen maskot penceresi (M2.3):** Şeffaf, çerçevesiz, her zaman üstte ve `WS_EX_NOACTIVATE` korumalı `mascot` penceresi; sürükleyip konum hatırlama; tek tık (Arama Adası) / çift tık (Uzman Arayüzü) / basılı tut (sıkma) / sağ tık kısa menü; `Ctrl+Alt+Space` göster/gizle; `Ctrl+Alt+F` arama adası; `Ctrl+Alt+S` uzman arayüzü; tam ekran uygulamalarda otomatik gizlenme.
- **Ada morph ve bulanık arama (M2.4):** Maskot ↔ hap ↔ arama adası yaylı morph geçişleri, `fuse.js` ile bulanık arama (10.000 ifadede < 30 ms), ↑↓ gezinme, sağ önizleme paneli, Esc ile kapanma ve Enter ile odak geri verilerek doğrudan yapıştırma.
- **Onay hapı ve yönetici rozeti (M2.5 & M2.6):** Genişletme sonrası `hop` + 1,2 sn onay hapı, hata durumunda `sad`, duraklatmada `sleep`; yükseltilmiş (yönetici) pencerede kalkan rozeti ve ipucu.

## [0.0.11] - 2026-11-26

### Düzeltilenler
- **Windows 11 Not Defteri (`notepad.exe`) uyumu:**
  - `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` + `QueryFullProcessImageNameW` kullanılarak paketlenmiş WinUI 3 uygulamalarının işlem adı güvenilir şekilde okunuyor.
  - Tüm sanal tuş gönderimlerinde (`VK_BACK`, `VK_RETURN`, `VK_TAB`, `Ctrl+V`) `MapVirtualKeyW` donanım tarama kodu (`wScan`) eklendi.
  - `notepad.exe` için kısa metinlerde dahi panoyu koruyarak yapıştırma (`CLIPBOARD_PREFERRED_PROCESSES`) ve Backspace sonrası kısa TSF yerleşme süresi eklendi.
  - Kanca iş parçacığında `GetKeyboardState` yerine `GetAsyncKeyState` / `GetKeyState` ile Shift, CapsLock ve AltGr durumu doğru okunuyor.

## [0.0.10] - 2026-11-26 (M1.0–M1.9 · Motor çekirdeği tamamlandı, M1.10 elle kabul testine hazır)

### Eklenenler
- **Proje iskeleti (M1.0):** Tauri 2 + React 19 + TypeScript (Vite) yapısı, §10 modül yerleşimi, Vitest ve `cargo test` kurulumu.
- **tr-TR harf kuralları (M1.1):** `tr_upper`, `tr_lower`, `tr_title`, `normalize_abbr`, `apply_case` (Yazışa uy / Aynen getir / Birebir eşleşme) ve isteğe bağlı noktasız/noktalı i eşitleme ayarı; çekirdekte standart `to_uppercase()` kullanılmadı.
- **Eşleme motoru (M1.2):** Bellek içi kısaltma indeksi, kelime başı kontrolü (`xMİPY` genişlemez), çok kelimeli kısaltma desteği (`İYİ GÜNLER`), §3.1 tetik tuşları, program kuralları arayüzü ve 10.000 ifadede < 1 ms arama testi.
- **SQLite katmanı (M1.3):** `rusqlite` (bundled), WAL modu, otomatik şema geçişi (`schema_migrations` v1 — §7.1 tüm tabloları), transaction içi CRUD, `deleted_at` çöp kutusu ve `%APPDATA%\Meetin\` / `portable.flag → .\data\` veri yolu seçimi.
- **Otomatik yedekleme (M1.4):** İlk açılışta ve kapanışta `VACUUM INTO backups\meetin-YYYY-MM-DD.db`, son 30 kopya rotasyonu.
- **Klavye ve fare kancası (M1.5):** Ayrı iş parçacığında `WH_KEYBOARD_LL` + `WH_MOUSE_LL`, `ToUnicodeEx` (`uFlags=0x4` ile ölü tuş koruması), 64 karakterlik yalnız-bellek arabelleği, `dwExtraInfo` imza koruması, dışlanan programlar ve UI Automation `IsPassword` koruması.
- **Metin yazma motoru (M1.6 & M1.7):** ≤ 200 karakter için pano kullanmadan `SendInput` + `KEYEVENTF_UNICODE`; > 200 karakter için tüm biçimleri yedekleyip geri yükleyen ve Win+V geçmişine girmeyen pano yapıştırma.
- **Tepsi simgesi ve tek örnek (M1.8):** `tauri-plugin-single-instance`, 32 px sadeleştirilmiş Meetin koyunu tepsi simgesi (etkin/duraklatılmış durumları), `Ctrl+Alt+P` duraklat/sürdür kısayolu.
- **CSV içe aktarma ve ifade listesi (M1.9):** UTF-8 / Windows-1254 ve `,` / `;` otomatik algılama, sütun eşleme, önizleme ve çakışma stratejileri (atla / üzerine yaz / yeni kısaltma öner), sade ifade yönetimi ekranı ve `src/i18n/tr.json`.
