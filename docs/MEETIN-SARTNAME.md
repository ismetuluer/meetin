# Meetin — Metin Genişletici · Ürün ve Teknik Şartname

> **Sürüm:** 0.3 · **Tarih:** 25 Kasım 2026 · **Hazırlayan:** Saul + Claude
> **Uygulayıcı:** Claude Code (bu belge proje klasörünün kökünde `docs/SARTNAME.md` olarak durur)
> **Program adı [K]:** **Meetin** ("mee" + "metin"). Maskot bir koyundur; kullanıcı ilk açılışta koyuna kendi adını verebilir (bkz. §5.1).
> **Ekler:** `meetin-maskot.svg` (maskotun ana çizimi) ve `meetin-maskot-animasyon.html` (tarayıcıda açılan, tüm animasyonların çalışan referansı). İkisi de `docs/` klasörüne konur.

İşaretler: **[K]** = Saul'un verdiği karar · **[Ö]** = Claude'un önerisi / varsayılanı (itiraz edilebilir) · **[A]** = uygulayıcıya bırakılan, ilk adımda doğrulanacak konu

---

## 1. Amaç

Windows'ta sık yazılan ifade kalıplarını kısaltmalarla çağırmak. `CİPY` yazıp boşluğa basınca yerine **"Claude ile Proje Yapmak"** gelir. PhraseExpress'in gücü, Apple'ın Dinamik Ada'sının sadeliği ve yumuşak, sevimli bir maskot bir arada olacak.

Kullanım yerleri **[K]**: haber/arşiv sistemleri ve tarayıcı formları, Word/Excel raporları, e-posta ve mesajlaşma (Outlook, WhatsApp Web, Telegram), kod ve yapay zekâ istemleri (Claude, terminal).

### 1.1 Hedefler

- Kısaltma yaz → tetik tuşu → metin anında ve doğru gelsin (Türkçe karakterler kusursuz).
- Mausla seçilen metin **tek hareketle** kütüphaneye girsin.
- Gündelik kullanım tamamen yüzen maskot ve ada üzerinden yürüsün. Uzman arayüzü sadece düzenleme ve yönetim için açılsın.
- Hem **portable** hem **kurulumlu** çalışsın. Arka planda sessizce dursun.
- Animasyonlar yaylı, yumuşak ve keyifli olsun; dikkat dağıtmasın.

### 1.2 Hedef dışı (bu sürümde yok)

- Ekip ile ortak kütüphane ve ağ veritabanı **[K: sadece ben]**.
- Bulut senkronizasyonu, hesap/oturum.
- macOS/Linux sürümü.
- Makro dili ya da SQL/betik çalıştıran ifadeler (PhraseExpress'teki "ProductID from SQL server" gibi).

### 1.3 "Çalışsa bile başarısız" sayılacak durumlar

- Genişletme sırasında harf kaçması, sıranın bozulması ya da Türkçe karakterin (ı, İ, ş, ğ, ç, ö, ü) yanlış gelmesi.
- Panonun (clipboard) kullanıcının haberi olmadan değişip eski hâline dönmemesi.
- Parola kutusunda yazılanların dinlenmesi ya da herhangi bir tuş vuruşunun diske/loga yazılması.
- Maskotun ya da adanın tam ekran video, sunum veya canlı yayın ekranının üstünde kalması.
- Boştayken işlemciyi meşgul etmesi, fanı çalıştırması.
- Veritabanının bozulması ya da bir ifadenin geri getirilemez şekilde kaybolması.
- Yarım kalmış özellik, işe yaramayan düğme, sahte veri (bkz. §16).

---

## 2. Temel kavramlar

| Terim                 | Anlamı                                                                                         |
| --------------------- | ---------------------------------------------------------------------------------------------- |
| **İfade**             | Kaydedilen metin (düz ya da biçimli). Açıklama, etiketler, klasör, kısaltma ve kurallar taşır. |
| **Kısaltma**          | İfadeyi çağıran harf dizisi (`CİPY`).                                                          |
| **Tetik tuşu**        | Kısaltmayı bitiren tuş: boşluk, Enter, Tab, nokta ve diğer noktalama işaretleri **[K]**.       |
| **Değişken**          | Genişletme anında doldurulan yer tutucu: tarih, saat, pano, imleç konumu.                      |
| **Doldurulacak alan** | Genişletmeden önce adanın sorduğu kullanıcı girdisi (`{alan:İsim}`).                           |
| **Maskot**            | Ekranda yüzen koyun. Programın ana giriş noktası.                                              |
| **Ada**               | Maskottan doğan, şekil değiştiren yüzen panel (arama, öneri, onay, ekleme, alan doldurma).     |
| **Balon**             | Metin seçilince imlecin yanında beliren küçük "+ Ekle" kabarcığı.                              |
| **Uzman arayüzü**     | Klasör ağacı, düzenleyici ve ayarları içeren tam pencere.                                      |

---

## 3. Genişletme motoru

### 3.1 Tetikleme **[K]**

- Kısaltmadan sonra **boşluk / Enter / Tab / . , ; : ! ? ) "** basılınca genişler. Tetik tuşu metnin sonuna eklenir: boşlukla tetiklendiyse sonda boşluk kalır, nokta ile tetiklendiyse nokta kalır.
- Kısaltma bir kelimenin **başından** başlamalı. `xCİPY` genişlemez **[Ö]**.
- **Geri al:** Genişletmeden hemen sonra (başka tuşa basmadan) **Backspace** basılırsa genişletme geri alınır ve kısaltma tekrar yazılmış hâle döner **[Ö]**.
- **Duraklat:** Maskotun sağ tık menüsünden ya da kısayolla bütün genişletmeler geçici olarak durur. Durunca maskot uyur.

### 3.2 Harf uyumu **[K: tercihe göre]**

Ayarlarda genel bir varsayılan seçilir, her ifadede bu varsayılan ayrıca değiştirilebilir:

| Mod                              | `cipy`                  | `Cipy`                  | `CİPY`                  |
| -------------------------------- | ----------------------- | ----------------------- | ----------------------- |
| **Yazışa uy** **[Ö varsayılan]** | claude ile proje yapmak | Claude ile proje yapmak | CLAUDE İLE PROJE YAPMAK |
| **Aynen getir**                  | Claude ile Proje Yapmak | Claude ile Proje Yapmak | Claude ile Proje Yapmak |
| **Birebir eşleşme**              | (açılmaz)               | (açılmaz)               | Claude ile Proje Yapmak |

**Türkçe kuralı (kritik):** Büyük/küçük dönüşümler **tr-TR** kurallarıyla yapılır: `i ↔ İ`, `ı ↔ I`. `cipy` ile `CİPY` eşleşir. `cıpy` ise `CIPY` ile eşleşir ve ikisi ayrı kısaltmadır. Rust tarafında varsayılan `to_uppercase()` kullanılmaz. Türkçe kurallı özel bir eşleme fonksiyonu yazılır ve birim testiyle korunur.

**[A]** İngilizce klavye düzeninde `CIPY` yazan biri için "noktasız/noktalı i'yi eşit say" seçeneği eklenmeli mi? Ayar olarak eklensin, varsayılan kapalı olsun **[Ö]**.

### 3.3 İçerik türleri **[K: hepsi]**

1. **Düz metin**: her programda çalışır.
2. **Biçimli metin**: kalın, italik, altı çizili, renk, başlık, liste, bağlantı. Word/Outlook gibi biçim kabul eden programlara biçimiyle, kabul etmeyenlere düz metin olarak gider.
3. **Değişkenler**: aşağıdaki sözdizimiyle.
4. **Doldurulacak alanlar**: genişletmeden önce ada küçük bir form olarak açılır.

**Değişken sözdizimi [Ö]:**

| Yazım                              | Sonuç                                                        |
| ---------------------------------- | ------------------------------------------------------------ |
| `{tarih}`                          | 23.11.2026                                                   |
| `{tarih:uzun}`                     | 23 Kasım 2026 Pazartesi                                      |
| `{tarih:+1g}` / `{tarih:-1a}`      | yarın / geçen ay (g=gün, h=hafta, a=ay, y=yıl)               |
| `{saat}`                           | 18:05                                                        |
| `{pano}`                           | panodaki metin                                               |
| `{imleç}`                          | genişletmeden sonra imlecin duracağı yer                     |
| `{alan:İsim}`                      | adada "İsim" kutusu açılır                                   |
| `{alan:Kanal\|A Haber,ATV,A News}` | açılır liste                                                 |
| `{ifade:KISALTMA}`                 | başka bir ifadeyi içine yerleştirir (döngü koruması zorunlu) |

Uzman düzenleyicisinde değişkenler, ekran görüntüsündeki PhraseExpress örneği gibi **renkli hap (chip)** olarak gösterilir, düz `{…}` yazısı görünmez. Hap, araç çubuğundaki "Değişken ekle" menüsünden eklenir.

### 3.4 Metni hedef programa yazma **[Ö]**

- **Kısa düz metin (≤ 200 karakter, biçimsiz):** kısaltma `SendInput` ile Backspace'lerle silinir, metin `SendInput` + `KEYEVENTF_UNICODE` ile yazılır. Pano hiç kullanılmaz.
- **Uzun ya da biçimli metin:** pano yedeklenir → HTML + RTF + düz metin biçimleri birlikte panoya konur → `Ctrl+V` → kısa bir gecikmeden sonra **pano eski hâline getirilir**. Pano geçmişi bu ara adımı kaydetmez.
- `{imleç}` varsa genişletmeden sonra gereken sayıda `←` tuşu gönderilir.
- Program bazlı yapıştırma yöntemi ayarlanabilir (bazı terminaller `Ctrl+Shift+V` ister).
- **[A]** Kurum içi haber/arşiv yazılımlarında `SendInput` ile pano yöntemleri ilk hafta gerçek ortamda denenecek. Gerekirse program bazlı istisna tablosu eklenecek.

### 3.5 Programa özel ifadeler **[K]**

Her ifadeye "sadece şu programlarda" ya da "şu programlarda hariç" kuralı verilebilir. Eşleme işlem adıyla (`WINWORD.EXE`) ve isteğe bağlı olarak pencere başlığındaki bir kelimeyle yapılır (ör. tarayıcıda "WhatsApp"). Aynı kısaltma farklı programlarda farklı ifadeye gidebilir.

### 3.6 Yönetici yetkisi

Windows, yönetici olarak çalışan bir programa normal yetkili programın tuş göndermesine izin vermez. Meetin normal yetkiyle çalışır. Etkin pencere yönetici yetkisindeyse maskot küçük bir kalkan rozeti gösterir ve "Bu pencerede çalışamıyorum" ipucu verir. Ayarlarda "Yönetici olarak başlat" seçeneği bulunur. Kurulumlu sürümde bu, Görev Zamanlayıcı ile UAC sormadan yapılır.

---

## 4. İfade ekleme yolları **[K: üçü de]**

### 4.1 Seçim balonu (PopClip tarzı)

- Mausla sürükleyerek, çift ya da üç tıklayarak metin seçilince imlecin yanında ~300 ms sonra küçük, vurgu renginde bir **"+ Ekle"** balonu belirir.
- Balona tıklanınca ada açılır. Metin önizlemesi gösterilir, kısaltma kutusu odaklanır, öneri kısaltma hazırdır (ifadenin baş harfleri: "Claude ile Proje Yapmak" → `CİPY`). Kullanıcı Enter'a basınca kaydedilir ve maskot ifadeyi "yutar".
- Balon 3 sn içinde kullanılmazsa ya da kullanıcı yazmaya devam ederse sessizce kaybolur.
- Seçili metin önce **UI Automation (TextPattern)** ile okunur. Program bunu desteklemiyorsa pano yedeklenir → `Ctrl+C` → okunur → pano geri yüklenir **[Ö]**.
- Ayar: balon kapatılabilir ve belirli programlarda gizlenebilir (ör. Photoshop, oyunlar).

### 4.2 Seç + kısayol

Varsayılan `Ctrl+Alt+E` **[Ö]**. Seçili metinle ada doğrudan ekleme modunda açılır.

### 4.3 Panodan

Adanın ekleme modunda ve pano geçmişinde "Panodakini ifade yap" düğmesi bulunur.

### 4.4 Ekleme adası alanları

Metin önizleme (biçimli metin biçimiyle korunur) · kısaltma (çakışma varsa anında uyarı) · açıklama (isteğe bağlı, otomatik doldurulur) · klasör (son kullanılan) · etiketler. **Enter** kaydeder, **Esc** vazgeçer. "Ayrıntılı düzenle" düğmesi uzman arayüzünü bu ifadede açar.

---

## 5. Maskot ve yüzen ada

### 5.1 Maskot: Meetin koyunu **[K]**

**Seçilen tasarım:** "Harman B · Bulut Gövde, Mochi Ruhu". Tasarım tuvalinde 22 numaralı çalışma. Birebir referans `docs/meetin-maskot.svg` ve `docs/meetin-maskot-animasyon.html` dosyalarıdır. Uygulamadaki maskot, bu dosyalardaki SVG yapısını, katman sınıf adlarını (`sh-all`, `legs`, `wool`, `head`, `ear-l`, `ear-r`, `eyes`, `eye-w`, `pupil`) ve CSS animasyonlarını aynen kullanır. Yeniden çizilmez.

**Görünüş:**

- **Gövde:** Tam karşıdan bakan, net koyu çizgili (`#2E2A28`, 3 px), kenarı dalgalı **beyaz bulut yün**.
- **Yüz:** Gövdenin önünde, yumuşak **kömür rengi yumurta** biçiminde.
- **Perçem:** Başın tepesinde küçük, dalgalı bir yün perçemi.
- **Kulaklar:** Yanlara açılan iki kara kulak.
- **Gözler:** Mochi'nin iri gözleri: beyaz oval, büyük siyah göz bebeği, iki ışık noktası.
- **Ağız ve yanaklar:** Ağız küçük ve açık renkli, yanaklarda hafif pembelik var.
- **Bacaklar:** Kısa ve kalın; öndeki ikisi kömür, arkadakiler biraz daha koyu.

**Karakter:** Sevimli ve meraklı. Tam bir hayvan gerçekçiliği yok; abartılı, sade şekillerden oluşur. Shaun the Sheep'ten esinlenilmiştir ama ona benzemez: kil dokusu yoktur, iki ayak üstünde durmaz, insan gibi el kol hareketi yapmaz.

**Renkler:** Klasik koyun renkleri kullanılır: beyaz, kömür, krem, hafif pembe. Maskotta **mor ya da lila kullanılmaz [K]**. Arayüzün renkleri temadan ve kullanıcının paletinden gelir (§11); maskot bunlardan etkilenmez.

**Fizik:** Sıkılabilir. Basılı tutunca yatayda genişleyip dikeyde ezilir (ölçek ≈ 1.26 × 0.72). Bırakınca yaylı bir şekilde (overshoot) zıplayarak eski hâline döner. Boşta yün hafifçe jöle gibi nefes alır.

**Her zaman üstte yüzer.** Sürüklenebilir ve yeri hatırlanır. Kısayolla (varsayılan `Ctrl+Alt+Space` **[Ö]**) ve sağ tık menüsünden gösterilip gizlenir. Tam ekran uygulamalarda kendiliğinden gizlenir **[Ö]**.

**Hareket eşlemesi [K: bu varsayılan, ayarlardan değiştirilebilir]:**

| Hareket                     | İşlev                                          |
| --------------------------- | ---------------------------------------------- |
| Tek tık                     | Arama adası                                    |
| Basılı tut + sık (≥ 450 ms) | Seçili metni ekle (seçim yoksa panodan ekleme) |
| Sürükle                     | Yerini değiştir                                |
| Çift tık                    | Uzman arayüzü                                  |
| Sağ tık                     | Kısa menü: Duraklat · Gizle · Ayarlar · Çıkış  |

#### 5.1.1 Animasyon kataloğu

Her animasyon `sheep a-<ad>` sınıfıyla tetiklenir. Yeni bir animasyon eklemek için bu tabloya bir satır, CSS'e bir `.a-<ad>` kuralı ve gerekiyorsa göz ya da ağız çeşidi eklenir **[K: animasyonlar zamanla eklenecek]**.

| Ad                 | Ne zaman                                         | Ne olur                                                                                                         | Süre                    |
| ------------------ | ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- | ----------------------- |
| `idle`             | Varsayılan                                       | Yün yavaşça nefes alır, gözler 4–5 sn'de bir kırpılır, kulaklar arada oynar, gözler imleci takip eder           | sürekli                 |
| `graze` (otla)     | Boşta kendiliğinden                              | Baş aşağı iner, ayakların dibinde çiçekli ot belirir, ağız çiğner                                               | 4,2 sn                  |
| `baa` (mele)       | Maskota tek tık (demo), boşta kendiliğinden      | Baş geriye kalkar, gövde titrer, ağız açılır, yanında "Mee!" balonu çıkar. Ses açıksa kısa bir mee sesi duyulur | 1,5 sn                  |
| `stare` (bana bak) | Kullanıcı uzun süre maskota yakın durunca, boşta | Gözler büyür, baş hafifçe öne gelir                                                                             | 2,8 sn                  |
| `sleep` (uyu)      | Duraklatıldığında, uzun süre boşta kalınca       | Gözler kapanır, kulaklar düşer, nefes yavaşlar, "zzz" uçar                                                      | durdurulana kadar       |
| `eat` (ifade ye)   | İfade kaydedilince                               | Küçük bir not kartı uçup ağza girer, yanaklar kızarır, yutkunur                                                 | 1,7 sn                  |
| `hop` (sevin)      | Kısaltma genişletilince                          | Mutlu "^ ^" gözler, iki kez zıplama                                                                             | 1,6 sn                  |
| `sad` (üzül)       | Bulunamadı ya da hata                            | Kulaklar sarkar, kaşlar düşer, baş iki yana sallanır                                                            | 1,5 sn                  |
| `squeeze` (sık)    | Basılı tutarken                                  | Gövde ezilir; bırakınca `eat` (basılı tutulduysa) ya da `baa` gelir                                             | basılı tutulduğu sürece |

- **Durum rozetleri [Ö]:** Güncelleme varsa sağ üstte vurgu renginde küçük bir nokta görünür. Etkin pencere yönetici yetkisindeyse küçük bir kalkan simgesi çıkar.
- **"Kendi hâlinde" modu:** Kullanıcı 5 sn etkileşimde bulunmazsa maskot 6 sn'de bir rastgele `graze`, `baa`, `stare` ya da `hop` oynatır. Ayarlardan kapatılabilir. `prefers-reduced-motion` açıkken bütün animasyonlar durur, maskot sadece göz kırpar.
- **Ses [K]:** Mee sesi **varsayılan kapalıdır**. Meleme her durumda görsel balonla gösterilir. Ses ayarlardan açılınca kısa ve yumuşak bir "mee" çalar. Örnek ses, referans dosyada Web Audio ile üretilir. Uygulamada küçük bir `.ogg` dosyası kullanılabilir **[A]**.
- **Boyutlar:** Maskot adada 70 px, ayrıca 48 px ve tepsi simgesi olarak 32 px boyutunda net okunmalıdır. Tepsi simgesi için kontur 2 px'e iner ve perçem sadeleştirilir **[Ö]**.
- **İlk açılış [K]:** Koyun uyanır (`sleep` → `hop`) ve "Mee! Bana bir isim verir misin?" diye sorar. Verilen ad koyunun adı olarak kullanılır; program adı her zaman **Meetin** kalır. Ardından 3 adımlık tanıtım gelir: kısaltma dene · metin seçip ekle · koyunu sık.
- **Çizim tekniği:** Maskot saf SVG ve CSS ile çizilir ve canlandırılır (transform/opacity). Three.js, Lottie ya da Rive kullanılmaz. Göz bebeği takibi tek bir `transform: translate()` ile yapılır. Maskot gizliyken bütün animasyonlar durdurulur.

### 5.2 Ada **[K: dört durumun hepsi]**

Ada maskottan "doğar". Maskot yatayda uzayarak hapa, hap da panele dönüşür (morph animasyonu). Kapanırken aynı yoldan geri döner.

| Durum              | Ne zaman                                                                         | İçerik                                                                                                                                  |
| ------------------ | -------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| **Arama**          | Tek tık ya da kısayol (`Ctrl+Alt+Space` iki kere ya da ayrı bir kısayol **[A]**) | Arama kutusu, bulanık arama (kısaltma + açıklama + içerik + etiket), ↑↓ ile gezme, Enter ile o anki programa yapıştırma, sağda önizleme |
| **Yazarken öneri** | Bir kısaltmanın ilk 2+ harfi yazılınca                                           | Maskotun altında 1–3 öneri hapı. `Tab` ile seçilir, yazmaya devam edilirse kaybolur. Ayardan kapatılabilir                              |
| **İşlem onayı**    | Genişletme, kaydetme, hata                                                       | 1,2 sn'lik hap: "✓ Kaydedildi · CİPY"                                                                                                   |
| **Alan doldurma**  | `{alan:…}` içeren ifade                                                          | Kompakt form. Tab ile alanlar arasında geçilir, Enter ile yazılır                                                                       |
| **Ekleme**         | §4                                                                               | Ekleme formu                                                                                                                            |

- Ada varsayılan olarak temaya uyar: açık temada açık, koyu temada koyu akrilik. Ayarlardan "Her zaman koyu" (Dinamik Ada görünümü) seçilebilir (§11.2).
- Klavye ile tamamen kullanılabilir. Esc her durumda kapatır.

---

## 6. Uzman arayüzü

PhraseExpress düzeninden esinlenilir ama kalabalık şerit menü kullanılmaz. Sade bir araç çubuğu ve komut paleti (`Ctrl+K`) olur.

- **Sol:** Klasör ağacı (sürükle-bırak, renk etiketi, iç içe klasör) + sabit girişler: _Son kullanılanlar · Pano geçmişi · Çöp kutusu_.
- **Orta:** İfade listesi (kısaltma, açıklama, son kullanım, kullanım sayısı). Sıralama ve filtre içerir.
- **Sağ:** Düzenleyici:
  - Açıklama + etiket hapları
  - Biçimli içerik düzenleyicisi (değişken hapları ile)
  - Tetikleyiciler: kısaltma (+ harf uyumu modu) ve isteğe bağlı **kısayol tuşu** (PhraseExpress'teki Ctrl+Alt+P gibi)
  - Kurallar: sadece/hariç programlar
  - Geçmiş: önceki sürümler, tek tıkla geri dön **[K]**
- **İstatistikler [K]:** En çok kullanılan 20 ifade, günlük/haftalık genişletme sayısı, **kazanılan tuş vuruşu ve tahmini dakika**. Maskot haftalık özet kartında "Bu hafta senin için 1.240 tuş bastım" der.
- **Pano geçmişi [K]:** Son 100 metin (ayarlanabilir). Parola yöneticilerinden gelen ve "gizli" işaretli pano içerikleri alınmaz. Tek tıkla ifadeye çevrilir.
- **İçe/dışa aktarma [K]:** bkz. §8.
- **Ayarlar:** Genel · Tetikleme · Harf uyumu · **Görünüm** (mod, palet kaynağı, ana renk, önerilen palet, tek tek düzenleme, ada yüzeyi, saydamlık; bkz. §11.2) · Maskot (hareketler, sesler, boyut, opaklık) · Kısayollar · Dışlanan programlar · Yedek · Güncelleme · Dil.

---

## 7. Veri ve güvenlik

### 7.1 Depolama **[Ö]**

- **SQLite** (WAL modu). Tüm yazmalar **işlem (transaction)** içinde yapılır. Şema sürümü tutulur ve geçişler (migration) otomatik çalışır.
- Tablolar: `phrases`, `phrase_versions`, `folders`, `tags`, `phrase_tags`, `app_rules`, `usage_events` (sadece ifade kimliği + zaman + program adı; **yazılan metin değil**), `clipboard_history`, `settings`.
- Silme: `deleted_at` alanı dolar (çöp kutusu). 30 gün sonra kalıcı silinir, bu süre ayarlanabilir **[K: çöp kutusu ve geçmiş]**.

### 7.2 Konumlar

- **Kurulumlu:** `%APPDATA%\Meetin\`
- **Portable:** EXE'nin yanında `portable.flag` dosyası varsa veriler `.\data\` klasöründe tutulur. Program USB'den başka bilgisayarda aynen çalışır **[K]**.

### 7.3 Yedek **[K]**

Her gün ilk açılışta ve kapanışta veritabanının tutarlı kopyası alınır (SQLite `VACUUM INTO`): `backups\meetin-2026-11-23.db`. Son 30 kopya tutulur. Ayarlardan "Şimdi yedekle" ve "Yedekten geri yükle" yapılabilir.

### 7.4 Gizlilik **[K]**

- **Parola kutularında** (UI Automation `IsPassword`) hiçbir tuş işlenmez, öneri gösterilmez.
- **Dışlanan programlar** listesi: o program etkinken klavye dinleyicisi tuşları sadece iletir, hiçbir şey tutmaz. Varsayılan liste: parola yöneticileri, uzak masaüstü **[Ö]**.
- Tuş arabelleği yalnızca bellekte tutulur, en fazla 64 karakterdir. Pencere değişince, fare tıklanınca ya da ok tuşlarına basılınca temizlenir. **Asla diske ya da loga yazılmaz.**
- Ağ erişimi **sadece** GitHub güncelleme kontrolü içindir. Telemetri yoktur.
- Şifreli veritabanı bu sürümde yoktur **[K]**.

---

## 8. İçe ve dışa aktarma **[K: tüm kaynaklar]**

| Kaynak             | Biçim                      | Not                                                                                                              |
| ------------------ | -------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Excel / CSV        | `.xlsx`, `.csv`            | Sütun eşleme ekranı: Kısaltma · Metin · Açıklama · Klasör · Etiketler. UTF-8 ve Windows-1254 otomatik algılanır  |
| Word / metin       | `.docx`, `.txt`            | Her paragraf ya da boş satırla ayrılmış blok bir ifade olur. Kısaltmalar sonra toplu atanır (öneri: baş harfler) |
| PhraseExpress      | dışa aktarma dosyası       | **[A]** Saul'dan örnek dosya alınıp biçim doğrulanacak                                                           |
| AutoHotkey         | `.ahk` hotstring satırları | `::cipy::Claude ile Proje Yapmak`                                                                                |
| Beeftext / Espanso | JSON / YAML                |                                                                                                                  |

- İçe aktarmadan önce **önizleme** gösterilir: kaç ifade, kaç çakışma. Çakışmada seçenekler: üzerine yaz · atla · yeni kısaltma öner.
- Dışa aktarma: `.meetin` paketi (ZIP içinde JSON + biçimli içerikler), CSV ve AHK.
- **Dağınık notlar için [K: "henüz dağınık"]:** "Hızlı toplu ekleme" ekranı. Bir metin kutusuna her satıra bir ifade yapıştırılır, program her biri için kısaltma önerir, kullanıcı tabloda düzeltip tek seferde kaydeder.

---

## 9. Güncelleme ve dağıtım **[K]**

- **Kurulumlu:** NSIS kurulum (`Meetin_x.y.z_x64-setup.exe`). Windows ile başlama seçeneği bulunur (varsayılan açık).
- **Portable:** `Meetin_x.y.z_portable.zip` (EXE + `portable.flag`).
- **Güncelleme:** Tauri Updater + **GitHub Releases** (`latest.json`, imzalı paketler). Açılışta ve 24 saatte bir kontrol edilir. Yeni sürüm varsa maskotta rozet görünür. Tıklanınca yenilikler listesi (CHANGELOG'dan) açılır, **"Şimdi kur"** ya da "Sonra" seçilir **[K: bildir, onayla kur]**.
- Portable sürüm güncellemesi: yeni EXE indirilir, eskisi `Meetin.old.exe` olur, yeniden başlatılır. `data\` klasörüne dokunulmaz.
- **[A]** WebView2: Windows 10/11'de normalde yüklüdür. Kurulumda yoksa çevrimdışı yükleyici eklenir. Portable sürümde yoksa açılışta uyarı verilir.
- **[A]** İmza: kod imzalama sertifikası yoksa SmartScreen uyarısı çıkar. README'de not olarak belirtilir.

---

## 10. Mimari **[K: Tauri + React]**

```
pamuk/
├─ src-tauri/                Rust çekirdek
│  ├─ hook/                  WH_KEYBOARD_LL + WH_MOUSE_LL, tuş arabelleği, tetik
│  ├─ matcher/               kısaltma eşleme, tr-TR harf kuralları, program kuralları
│  ├─ inject/                SendInput, pano yedekle/yapıştır/geri yükle, HTML/RTF
│  ├─ selection/             UI Automation ile seçili metin, Ctrl+C yedeği
│  ├─ vars/                  değişken ve alan çözümleyici
│  ├─ db/                    SQLite, migration, yedek, çöp kutusu
│  ├─ io/                    içe/dışa aktarma
│  └─ windows/               pencere yönetimi (maskot, ada, balon, uzman)
├─ src/                      React + TypeScript
│  ├─ mascot/                SVG maskot + durum makinesi + yay fiziği
│  ├─ island/                ada durumları (arama, öneri, onay, alan, ekleme)
│  ├─ bubble/                seçim balonu
│  ├─ studio/                uzman arayüzü
│  ├─ design/                renk/ölçü/hareket tokenları
│  └─ i18n/tr.json           tüm arayüz metinleri (en.json'a hazır)
└─ docs/  SARTNAME.md · CHANGELOG.md · HANDOFF.md
```

- **Pencereler:** (1) maskot + ada: şeffaf, çerçevesiz, her zaman üstte, odak çalmaz (`WS_EX_NOACTIVATE`); arama modunda odak alır. (2) balon: şeffaf, tıklama geçirgen olmayan küçük pencere. (3) uzman arayüzü: normal pencere, Mica arka planlı.
- Klavye kancası ayrı bir iş parçacığında çalışır ve **hiçbir zaman bloklamaz**. Kanca geri çağrısı 1 ms altında döner, ağır iş kanal (channel) ile başka iş parçacığına verilir.
- **Kütüphane önerileri [Ö]:** `windows` crate, `rusqlite`, `tauri-plugin-updater`, `tauri-plugin-global-shortcut`, `tauri-plugin-single-instance`; arayüzde `motion` (yay animasyonları), `@tiptap/react` (biçimli düzenleyici), `fuse.js` (bulanık arama). Bunun dışında yeni bağımlılık eklemek için gerekçe yazılır.

### 10.1 Performans hedefleri **[Ö]**

- Tetikten metnin görünmesine: **< 50 ms** (kısa düz metin).
- Boşta işlemci: **~%0** (boşta animasyonlar `requestAnimationFrame` yerine CSS ile yapılır; maskot gizliyken tamamen durur).
- Bellek: **< 120 MB** toplam (WebView2 dahil).
- 10.000 ifadede arama: **< 30 ms**.

---

## 11. Görsel dil, tema ve renk paleti **[K: Windows temasıyla uyumlu, palet kullanıcıya açık]**

**İlke:** Meetin, Windows 11'in kendi uygulamaları gibi görünür ve davranır. Varsayılan olarak Windows'un açık/koyu modunu ve **vurgu rengini** izler. Görünüş sade ve Apple tarzı minimal kalır. Renkleri kullanıcı üç düzeyde değiştirebilir (§11.2). Maskot bunların hiçbirinden etkilenmez; her zaman klasik koyun renklerinde kalır (§5.1).

### 11.1 Windows ile uyum

- **Açık/koyu mod:** Varsayılan **Sistem** modudur. Windows'ta "Uygulama modu" değişince Meetin uygulamayı yeniden başlatmadan, 200 ms'lik yumuşak bir geçişle temasını değiştirir. Elle "Açık" ya da "Koyu" da seçilebilir.
- **Vurgu rengi:** Windows vurgu rengi okunur (Rust: `windows` crate · `UISettings.GetColorValue(UIColorType::Accent)` ve `ColorValuesChanged` olayı). Bu renk değişince palet anında yeniden üretilir.
- **Yüzey malzemesi:** Uzman arayüzü penceresinde Windows 11 **Mica**, ada ve balonda **Akrilik** kullanılır (`window-vibrancy` crate). Windows 10'da ya da "Saydamlık efektleri" kapalıyken düz, yarı opak renk kullanılır.
- **Yazı:** Segoe UI Variable, yoksa Segoe UI. Simgeler Fluent stilinde, ince çizgili.
- **Köşe ve boşluklar:** Windows 11 ölçülerine uyulur: pencere 8 px, kart ve ada 12–24 px köşe, 4 px'lik boşluk ızgarası.
- **Yüksek karşıtlık:** Windows "Karşıtlık temaları" açıksa (`forced-colors`) özel palet devre dışı kalır ve sistem renkleri kullanılır. Maskot da sadece kontur çizgileriyle gösterilir **[Ö]**.
- **Animasyon efektleri:** Windows'ta "Animasyon efektleri" kapalıysa ada geçişleri kısa bir solmaya iner. Maskot sadece göz kırpar.

### 11.2 Renk paletini değiştirme

Ayarlar › **Görünüm** sayfasında, yukarıdan aşağıya üç düzey vardır:

1. **Palet kaynağı:**
   - **Sistemle uyumlu** (varsayılan): Ana renk Windows vurgu rengidir, açık/koyu mod sistemi izler.
   - **Ana renk seç:** Renk tekerleği, hex kutusu ve hazır renkler bulunur. Varsayılan hazırlar: Mor `#7C4DFF`, Okyanus, Orman, Kehribar, Gül, Grafit.
2. **Önerilen palet:** Seçilen ana renkten bütün arayüz renkleri otomatik üretilir (§11.3). Sağda **canlı önizleme** görünür: ada (arama + onay hapı), seçim balonu, düğmeler, seçili satır ve maskot. Değişiklik aynı anda gerçek adaya da yansır.
3. **Gelişmiş: tek tek düzenle:** Her renk rolü (§11.4) bir satırdır: renk kutusu, hex değeri, **karşıtlık rozeti** (AA ✓ / ✗) ve "Öneriye dön" düğmesi. Elle değiştirilen roller "elle" etiketi taşır. Ana renk sonradan değişirse elle değiştirilenler korunur, diğerleri yeniden önerilir.

Ayrıca:

- Mod seçimi (Sistem / Açık / Koyu) ve "Ada yüzeyi" seçeneği bulunur: **Temaya uyar** (varsayılan) ya da **Her zaman koyu** (Dinamik Ada görünümü).
- Akrilik saydamlığı için bir kaydırıcı vardır.
- **Tema dışa/içe aktarma:** `.meetin-tema` dosyası (JSON) olarak yapılır.
- **Varsayılana dön** düğmesi bulunur.

### 11.3 Önerilen paletin üretimi

Ana renkten (A) palet algısal olarak eşit bir renk uzayında (**OKLCH**) üretilir **[Ö]**:

- **Vurgu ölçeği:** A'nın tonu (H) ve doygunluğu (C) korunur. 11 basamak üretilir (50…950), açıklık (L) 0,97'den 0,25'e iner. Ekran dışına taşan renklerde C kırpılır.
- **Nötrler:** Zemin, yüzey ve çizgi renkleri A'nın tonundan çok hafif (C ≈ 0,01) renk alır. Böylece arayüz "ana renge ait" hisseder ama gri kalır.
- **Vurgu üstü metin:** Vurgu renginin üstündeki yazı beyaz ya da koyu olur; WCAG karşıtlığı yüksek olan seçilir.
- **Durum renkleri:** Başarı, uyarı ve hata sabit tonlarda kalır (yeşil, kehribar, kırmızı). Açıklıkları ana renkle uyumlanır.
- **Koyu tema:** Aynı ton farklı L değerleriyle kullanılır; vurgu bir basamak açılır.
- **Karşıtlık güvencesi:** Üretilen her metin/zemin çifti en az **4,5:1** olmalıdır. Tutmuyorsa L otomatik kaydırılır. Kullanıcının elle seçtiği renk bu oranı tutmuyorsa engellenmez, ama satırda uyarı rozeti çıkar.
- Hesaplama arayüz tarafında küçük bir saf TypeScript modülünde yapılır, ek kütüphane kullanılmaz. Birim testleriyle korunur.

### 11.4 Renk rolleri (CSS değişkenleri)

| Rol                                             | Kullanım                                                                           |
| ----------------------------------------------- | ---------------------------------------------------------------------------------- |
| `--vurgu` / `--vurgu-hover` / `--vurgu-basili`  | Ana düğme, seçim balonu, odak halkası, seçili sekme                                |
| `--vurgu-ustu`                                  | Vurgu zeminindeki yazı ve simgeler                                                 |
| `--vurgu-zayif`                                 | Seçili satır, değişken hapları, kısaltma çipi                                      |
| `--zemin` / `--yuzey` / `--yuzey-2`             | Pencere zemini, kartlar, ada ve iç kutular                                         |
| `--cizgi`                                       | Ayraçlar, kenarlar                                                                 |
| `--metin` / `--metin-ikincil` / `--metin-pasif` | Metin seviyeleri                                                                   |
| `--basari` / `--uyari` / `--hata`               | Onay hapı, uyarılar, hata durumu                                                   |
| `--ada-zemin` / `--ada-metin`                   | Ada; "Her zaman koyu" seçiliyse sabit koyu değerler                                |
| `--koyun-*`                                     | Maskot renkleri. **Kullanıcı değiştiremez [K]**; yüksek karşıtlık dışında sabittir |

Bütün bileşenler yalnızca bu rolleri kullanır. Bileşen içinde sabit renk yazılmaz.

### 11.5 Hareket

- Yay (spring) temel değerleri: sertlik 380, sönüm 28. Ada açılışı ~350 ms, kapanış ~220 ms.
- Animasyonlar sadece `transform` ve `opacity` üzerinden yapılır (60 fps).
- Windows "Animasyon efektleri" kapalıysa (`prefers-reduced-motion`) yaylar kısa geçişlere döner, maskot sadece göz kırpar.
- Esinlenme: designbookmark.com/motion'daki _Morph island, Goo morph, Jelly slider, Springy input, Loading to success, Copy button, Siri orb_ örnekleri.

### 11.6 Tasarım kaynakları

- **Tasarım tuvali:** https://claude.ai/artifact/XdbvzgCZMpGKceVg2XwBcR. 01–05 numaralı ada çalışmaları **düzen ve hareket** için referanstır: ada biçim değiştirerek açılır, arama, ekleme ve onay hapı. Renkleri bağlayıcı değildir; renkler §11.2–11.4'e göre temadan gelir.
- **Maskot:** Harman B (22). Bkz. §5.1.

## 12. Varsayılan kısayollar **[Ö]**

| Kısayol          | İşlev                          |
| ---------------- | ------------------------------ |
| `Ctrl+Alt+Space` | Maskotu göster/gizle           |
| `Ctrl+Alt+F`     | Arama adası                    |
| `Ctrl+Alt+E`     | Seçili metni ifade olarak ekle |
| `Ctrl+Alt+P`     | Genişletmeyi duraklat/sürdür   |
| `Ctrl+Alt+S`     | Uzman arayüzü                  |

Hepsi ayarlardan değiştirilebilir. Çakışma olursa (başka bir program aynı kısayolu kullanıyorsa) uyarı verilir.

---

## 13. Geliştirme aşamaları

Her aşama kullanılabilir, test edilmiş, sürüm numarası verilmiş bir hâlde biter.

| Aşama                         | İçerik                                                                                                                                                       | Bitti sayılması için                                          |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------- |
| **M1 · Motor** (v0.1)         | Klavye kancası, eşleme (tr-TR), düz metin genişletme, SQLite, tepsi simgesi, CSV içe aktarma                                                                 | `CİPY␣` Notepad, Word, Chrome, Outlook'ta doğru çalışıyor     |
| **M2 · Maskot ve ada** (v0.2) | Meetin koyunu (referans SVG'den, animasyon kataloğuyla), sürükleme, göster/gizle, arama adası, onay hapı                                                     | Maskotla arayıp yapıştırma çalışıyor                          |
| **M3 · Ekleme** (v0.3)        | Seçim balonu, seç+kısayol, panodan ekleme, sıkma hareketi                                                                                                    | Üç yol da Word, Chrome ve kurum yazılımında çalışıyor         |
| **M4 · Zengin içerik** (v0.4) | Biçimli metin, değişkenler, alan doldurma, `{imleç}`, geri al                                                                                                | Biçimli ifade Word/Outlook'a biçimiyle gidiyor                |
| **M5 · Uzman arayüzü** (v0.5) | Klasör/etiket, düzenleyici, kurallar, geçmiş, çöp kutusu, pano geçmişi, istatistik, içe/dışa aktarma                                                         | PhraseExpress'teki temel işler yapılabiliyor                  |
| **M6 · Dağıtım** (v1.0)       | Görünüm ayarları (palet üretici, tek tek düzenleme, tema dosyası), NSIS + portable, GitHub güncelleme, ilk açılış ve isim verme, yedek, ayarlar, öneri adası | Temiz bir Windows'ta kurulum → güncelleme → kaldırma sorunsuz |

---

## 14. Test listesi

**Otomatik:** palet üretici (her ana renkte 4,5:1 karşıtlık, açık ve koyu tema), tr-TR harf eşleme (i/İ/ı/I tüm kombinasyonlar), değişken çözümleyici, `{ifade:}` döngü koruması, DB migration, içe aktarma (Türkçe karakterli CSV, 1254 kodlaması, boş satırlar, tırnaklı alanlar), yedek/geri yükleme.

**Elle (her aşama sonunda):**

- Tema: Windows açık↔koyu geçişi ve vurgu rengi değişimi (uygulama açıkken), saydamlık kapalı, yüksek karşıtlık teması, Windows 10.
- Programlar: Notepad, Word, Excel (hücre içinde ve formül çubuğunda), Outlook, Chrome/Edge (WhatsApp Web, Gmail), Telegram masaüstü, Windows Terminal, VS Code, kurum içi haber/arşiv yazılımı.
- Durumlar: hızlı yazma (100+ kelime/dk), çok uzun ifade (10.000 karakter), boş ifade, emoji, sağdan sola metin, iki monitör ve farklı DPI (%100/%150), uyku/uyanma, kilit ekranı, yönetici olarak çalışan pencere, tam ekran video, oyun, uzak masaüstü, parola kutusu, pano yöneticisi açıkken, uzun süre (24 saat) açık kalma ve bellek sızıntısı.
- Veri: genişletme sırasında programı zorla kapatma → DB sağlam mı; disk dolu; salt okunur USB'de portable.

---

## 15. Uygulayıcıya bırakılanlar ve açık sorular

1. **[A]** PhraseExpress dışa aktarma biçimi: Saul örnek dosya verecek.
2. **[A]** Kurum içi yazılımda en güvenilir yazma yöntemi (§3.4): ilk hafta gerçek ortamda denenecek.
3. **[A]** Öneri adasının kaç harften sonra açılacağı (varsayılan 2) ve dikkat dağıtıp dağıtmadığı: M6'da kullanımla ayarlanacak.
4. **[A]** Arama adası kısayolunun ayrı mı yoksa göster/gizle ile ortak mı olacağı.
5. **Karar verildi:** Program adı **Meetin**, maskot **Harman B** koyunu.
6. **Karar verildi:** Arayüz Windows temasına uyar. Palet sistemden alınır ya da bir ana renkten önerilir, ayrıca tek tek düzenlenebilir (§11).
7. **[A]** "Meetin" adının GitHub'da ve marka kayıtlarında boşta olup olmadığı kontrol edilecek.

---

## 16. Claude Code için çalışma kuralları

- Her anlamlı adım sonunda **sürüm numarası** artırılır ve `CHANGELOG.md` gerçek değişikliklerle güncellenir.
- Bir özellik ancak tüm zincir çalışınca **bitti** sayılır: arayüz → etkileşim → iş mantığı → veri → hata durumu → kullanıcıya geri bildirim → test.
- Teknik borç bırakılmaz: geçici çözüm, sahte veri, ölü kod, işe yaramayan düğme, sessizce yutulan hata, açıklamasız TODO olmaz. Bitirilemeyen iş `HANDOFF.md`'de açıkça yazılır.
- Test edilmeyen bir şey "test edildi" diye raporlanmaz.
- Önceki yapay zekâ raporlarına körü körüne güvenilmez; gerçek dosya ve davranış kontrol edilir.
- **Aynı anda en fazla bir alt ajan** çalıştırılır; kota tasarrufu önceliklidir.
- Bağlam sınırına yaklaşınca `HANDOFF.md` yazılır: durum, yığın, bitenler, bilinen hatalar, teknik borç, sıradaki önerilen iş.
- Saul kısa "Devam" komutu verdiğinde: mevcut durumu anla → en önemli kalan işi seç → yap → test et → gerilemeyi kontrol et → arayüzü kontrol et → sürüm + CHANGELOG → rapor + sıradaki adım.
