<p align="center">
  <img src="docs/screenshots/meetin-mascot.png" alt="Meetin Maskotu" width="112" />
</p>

<h1 align="center">Meetin · Akıllı Türkçe Metin Genişletici (<code>v1.2.0</code>)</h1>

<p align="center">
  Windows için tasarlanmış yerel, hızlı, gizlilik odaklı ve Türkçe (<code>tr-TR</code>) uyumlu akıllı metin genişletici ve etkileşimli masaüstü koyun maskotu.
</p>

---

## Ekran Görüntüleri

<p align="center">
  <img src="docs/screenshots/meetin-studio.png" alt="Meetin Ana Arayüz" width="48%" />
  &nbsp;
  <img src="docs/screenshots/meetin-appearance.png" alt="Meetin Görünüm ve Tema Motoru" width="48%" />
</p>

- **Masaüstü Koyun Maskotu ([meetin-maskot.svg](docs/meetin-maskot.svg)):** Sürüklenebilir, fırlatılabilir, tıklayınca arama adası ve seçili metinlerde anında `+ Ekle` balonu açan; sağ tıkla tek adımda **Sessize Al** / **Duraklat** seçenekleri sunan üretkenlik maskotu.

---

## Öne Çıkan Özellikler

- **Türkçe Büyük/Küçük Harf Uyumu (`tr-TR`):** `i ↔ İ`, `ı ↔ I` ayrımını bozmadan *Aynen getir* (varsayılan), *Harf uyumu* ve *Birebir eşleşme* modlarıyla genişletme + *Aksanları yoksay* seçeneği.
- **Kesintisiz `Shift+F3` Harf Döngüsü:** Hangi düzenlenebilir uygulamada olursan ol seçili metni Word tarzı `küçük → İlk harf büyük → TÜMÜ BÜYÜK` döngüsüyle dönüştürür ve seçimi korur; ardı ardına `Shift+F3` basarak devam edebilirsiniz.
- **Güçlü Kütüphane ve Klasör Yönetimi:**
  - Tek ifadeyi birden fazla klasörde bulundurma, klasörler arası serbest taşıma veya `Ctrl+sürükle` ile kopyalama.
  - **Toplu seçme**, sağ tık bağlam menüsü, klasörlere 8 farklı renk atama ve yeniden adlandırma.
  - İfadeyi **Çöp Kutusu**na sürükleyerek veya listede sola kaydırıp kırmızı **Sil** ile anında silme.
  - Kütüphanenin tamamını, bir klasörü ya da filtrelenmiş listeyi `.mee`, `CSV` veya `.ahk` olarak dışa aktarma.
- **Akıllı Otomatik İşlemler:** 650 ms otomatik kaydetme, içerik yazıldıkça otomatik açıklama doldurma (elle müdahale edilene dek), klasör adını etikete otomatik ekleme ve ayarlanabilir kısaltma önerisi (*İlk harfler*, *İlk kelime*, *Son kelime*, *Etiket*).
- **Pano Geçmişi ve Biçim Dönüştürücü:** Ayarlanabilir kayıt sayısı (varsayılan son 10 kopya); kayıtlı her metni tek tıkla *Kopyala*, *TÜMÜ BÜYÜK*, *tümü küçük*, *İlk Harf Büyük* olarak kopyalama.
- **Ayrı Ses Kontrolleri:** `Ayarlar › Maskot` üzerinden ana ses, *Mee sesi*, *Fırlatma sesi (whoosh)* ve *Duvar çarpma sesi* ayrı ayrı açılıp kapatılabilir.
- **Tema ve Yazı Boyutu Özelleştirme:** Windows açık/koyu modu ve vurgu rengini canlı izler; WCAG ≥ 4,5:1 karşıtlık güvenceli OKLCH paleti, `.mee-tema` paylaşımı ve **Küçük / Normal / Büyük / Çok Büyük** yazı boyutu sunar.
- **Güvenilir Yedekleme (`%APPDATA%\Meetin`):** Tüm veriler kullanıcı `AppData\Roaming\Meetin\` klasöründe tutulur; açılışta ve kapanışta otomatik SQLite `VACUUM INTO` yedeği (30 kopya rotasyonu) alınır.

---

## Varsayılan Kısayollar

| Kısayol | İşlev |
| --- | --- |
| `Ctrl + Alt + F` | Arama adasını aç |
| `Ctrl + Alt + E` | Seçili metinden hızlı ekle |
| `Ctrl + Alt + S` | Ana pencereyi aç |
| `Ctrl + Alt + P` | Motoru duraklat / sürdür |
| `Ctrl + Alt + Space` | Maskotu göster / gizle |
| `Ctrl + K` | Ana pencerede Komut Paleti |
| `Shift + F3` | Seçili metnin büyük/küçük harf biçimini değiştir (ardı ardına döngü) |

---

## İndirme ve Kurulum

- **Kurulumlu (NSIS):** [GitHub Releases](https://github.com/ismetuluer/meetin/releases/latest) üzerinden `Meetin_1.2.0_x64-setup.exe` indirip kurabilirsiniz.
- **Taşınabilir (Portable):** `Meetin_1.2.0_portable.zip` kurulum gerektirmeden çalışır ve verileri varsayılan olarak `%APPDATA%\Meetin\` altında saklar (yerel `.\data\` klasörüne almak için EXE yanına boş `local-data.flag` dosyası eklenebilir).
- **Otomatik Güncelleme:** Meetin yeni sürümleri doğrudan GitHub Releases (`v*` etiketi iş akışı: [release.yml](.github/workflows/release.yml)) üzerinden denetler.
- **Geliştirme başlatma:** Projeden doğrudan çalıştırmak için [Start.bat](Start.bat) dosyasını kullanabilirsiniz.
- **Atlanan Özellikler Yol Haritası:** Gelecek sürümler için planlanan özellikler [docs/ATLANANLAR_YOL_HARITASI.md](docs/ATLANANLAR_YOL_HARITASI.md) dosyasında yer alır.

---

## Windows SmartScreen Uyarısı (§9)

Meetin açık kaynaklı ve kod imzalama sertifikası olmadan dağıtıldığı için Windows Defender SmartScreen ilk çalıştırmada *"Windows kişisel bilgisayarınızı korudu"* uyarısı gösterebilir. Devam etmek için **Ek bilgi** → **Yine de çalıştır** adımını seçebilirsiniz.
