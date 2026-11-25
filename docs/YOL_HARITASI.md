# Meetin · Yol Haritası

Kaynak: `docs/MEETIN-SARTNAME.md` (§13 aşamaları). Her adım tek commit (gerekirse birkaç küçük commit) ile biter.
**Sürüm:** Aşama içi adımlar yama numarasını artırır (`0.0.x`, `0.1.x` …); aşama kabul testi geçince küçük sürüm çıkar (`0.1.0` = M1).
**Durum işaretleri:** `[ ]` başlanmadı · `[~]` yarım · `[x]` tamamlandı (yalnızca dosya + test ile doğrulanınca).
**Test:** _Oto_ = `cargo test` / `npm test` ile çalışan · _Elle_ = Saul'un yapacağı kontrol.
**Ajan:** Opus = ana oturum · Sonnet / Haiku = alt ajan (aynı anda tek). Opus, alt ajan çıktısını her zaman dosyada ve testle doğrular.

---

## M1 · Motor → v0.1.0

Bitti ölçütü (§13): `CİPY␣` Notepad, Word, Chrome, Outlook'ta doğru çalışıyor.

### [~] M1.0 · Proje iskeleti — v0.0.1

- **Hedef:** `git init`, Tauri 2 + React + TS (Vite) iskeleti, §10 klasör yapısı (Rust modülleri `src-tauri/src/<modül>/`), `.gitignore`, `CHANGELOG.md`, vitest + `cargo test` kurulumu, rustfmt/clippy/eslint/prettier.
- **Modüller:** kök, `src-tauri/`, `src/`
- **Bitti:** `npm run tauri dev` boş pencere açar; `cargo test`, `npm test`, `cargo clippy -- -D warnings` temiz.
- **Test:** Oto: boş test takımları geçer · Elle: pencere açılıyor mu.
- **Ajan:** Opus.

### [ ] M1.1 · tr-TR harf kuralları — v0.0.2

- **Hedef:** `tr_upper`, `tr_lower`, `tr_title`; kısaltma normalize fonksiyonu; üç harf uyumu modu (yazışa uy / aynen / birebir) ve çıktıya harf uygulama. "Noktasız/noktalı i eşit" seçeneği (varsayılan kapalı).
- **Modüller:** `matcher/case.rs`
- **Bitti:** §3.2 tablosunun tüm hücreleri testle doğrulanıyor; `to_uppercase()` çekirdekte hiç kullanılmıyor (grep ile).
- **Test:** Oto: i/İ/ı/I tüm kombinasyonlar, §3.2 tablosu, karışık harfli girişler.
- **Ajan:** Opus.

### [ ] M1.2 · Eşleme motoru — v0.0.3

- **Hedef:** Bellekte kısaltma indeksi; "kelime başında başlar" kuralı; tetik kümesi (boşluk, Enter, Tab, `. , ; : ! ? ) "`); aynı kısaltmanın program kuralına göre seçimi için arayüz (kural verisi M4.5'te dolacak).
- **Modüller:** `matcher/`
- **Bitti:** Tuş arabelleği + tetik → eşleşen ifade / yok kararı saf fonksiyonla veriliyor; 10.000 ifadede arama < 1 ms.
- **Test:** Oto: kelime başı (`xCİPY` açılmaz), her tetik tuşu, mod etkileşimi, benchmark testi.
- **Ajan:** Opus.

### [ ] M1.3 · SQLite katmanı — v0.0.4

- **Hedef:** `rusqlite` (bundled), WAL, migration altyapısı + v1 şeması (§7.1'deki tüm tablolar), ifade CRUD (her yazma transaction içinde), veri konumu (`%APPDATA%\Meetin\` / `portable.flag` → `.\data\`).
- **Modüller:** `db/`
- **Bitti:** Boş DB'den v1'e geçiş; ikinci çalıştırmada değişiklik yok; silme `deleted_at` ile.
- **Test:** Oto: migration, CRUD, transaction geri alma, portable yol seçimi (geçici klasörde).
- **Ajan:** Opus.

### [ ] M1.4 · Otomatik yedek (öne alındı, bkz. not) — v0.0.5

- **Hedef:** Günün ilk açılışında ve kapanışta `VACUUM INTO backups\meetin-YYYY-MM-DD.db`; son 30 kopya. (Geri yükleme arayüzü M6.5'te.)
- **Modüller:** `db/backup.rs`
- **Bitti:** Yedek dosyası açılabilir ve içerik birebir; 31. yedek en eskiyi siler.
- **Test:** Oto: yedek al → aç → karşılaştır; rotasyon.
- **Ajan:** Opus.

### [ ] M1.5 · Klavye kancası — v0.0.6

- **Hedef:** Ayrı iş parçacığında mesaj döngülü `WH_KEYBOARD_LL` + `WH_MOUSE_LL`; geri çağrı < 1 ms, iş kanal ile aktarılır; karakter çözümü düzen-bağımsız (`ToUnicodeEx`, durum değiştirmeyen bayrakla; ölü tuşlar bozulmaz); 64 karakterlik yalnız-bellek arabelleği; pencere değişimi / fare tıkı / ok tuşlarında temizleme; kendi enjekte ettiğimiz olayları yok sayma (`dwExtraInfo` imzası); parola kutusu (`IsPassword`) ve dışlanan programlarda tam geçiş.
- **Modüller:** `hook/`
- **Bitti:** Arabellek ve temizleme kuralları saf fonksiyonda; hiçbir tuş diske/loga yazılmıyor (kod incelemesi + grep).
- **Test:** Oto: arabellek/temizleme mantığı · Elle: Notepad'de hızlı yazarken gecikme yok, Türkçe Q ve F klavyede ölü tuşlar (`^`, `¨`) bozulmuyor, parola kutusunda tepki yok.
- **Ajan:** Opus.

### [ ] M1.6 · SendInput ile kısa metin yazma — v0.0.7

- **Hedef:** Tetik tuşu yutulur → kısaltma Backspace ile silinir → metin `KEYEVENTF_UNICODE` ile yazılır → tetik tuşu sona eklenir. Enjeksiyon sırasında kullanıcının bastığı tuşlar sıraya alınıp sonra iletilir (sıra bozulmaz). Duraklat (`Ctrl+Alt+P` geçici; kısayol kararı M2'de).
- **Modüller:** `inject/`, `hook/`, `matcher/`
- **Bitti:** `CİPY␣` Notepad'de doğru; tetik < 50 ms.
- **Test:** Oto: olay dizisi üretimi (saf fonksiyon) · Elle: Notepad, Word, Chrome, Outlook; 100+ kelime/dk hızlı yazma; Enter ile tetik (WhatsApp Web'de mesaj kısaltmayla gitmiyor), Excel hücresi.
- **Ajan:** Opus.

### [ ] M1.7 · Pano ile uzun metin — v0.0.8

- **Hedef:** > 200 karakter: panoyu tüm biçimleriyle yedekle → metni pano geçmişine girmeyecek şekilde koy (`ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory=0`) → `Ctrl+V` → gecikmeli geri yükle.
- **Modüller:** `inject/clipboard.rs`
- **Bitti:** 10.000 karakterlik ifade doğru geliyor; önceki pano içeriği (resim dahil) geri geliyor; Win+V geçmişinde ara içerik yok.
- **Test:** Oto: biçim yedek/geri yükleme (gerçek pano üstünde, seri test) · Elle: Word, Chrome, Windows Terminal; Win+V kontrolü; açık pano yöneticisiyle.
- **Ajan:** Opus.

### [ ] M1.8 · Tepsi simgesi ve tek örnek — v0.0.9

- **Hedef:** `tauri-plugin-single-instance`; tepsi simgesi (referans SVG'den 32 px, kontur 2 px, perçem sade) ve menü: Duraklat · Çıkış. Duraklatınca simge değişir.
- **Modüller:** `windows/tray.rs`, `src-tauri/icons/`
- **Bitti:** İkinci başlatma mevcut örneği öne getirir; menü tüm öğeleri çalışır.
- **Test:** Elle: %100 / %150 DPI'da simge netliği, duraklat/sürdür.
- **Ajan:** Opus (kod) · Sonnet (SVG'den sadeleştirilmiş tepsi simgesi + ICO üretimi).

### [ ] M1.9 · CSV içe aktarma + geçici ifade listesi — v0.0.10

- **Hedef:** CSV ayrıştırıcı (UTF-8 / Windows-1254 algılama, tırnaklı alan, boş satır); sütun eşleme ekranı; önizleme (kaç ifade, kaç çakışma) ve çakışma seçenekleri; yeni ifade ekleme/düzenleme için sade liste ekranı (M5'te uzman arayüzüne dönüşecek, ölü kod kalmaz). Arayüz metinleri `i18n/tr.json`'da.
- **Modüller:** `io/csv.rs`, `src/studio/` (ilk hâli), `src/i18n/tr.json`
- **Bitti:** Türkçe karakterli 1254 ve UTF-8 CSV'ler doğru içe aktarılıyor; eklenen ifade anında genişliyor.
- **Test:** Oto: §14 içe aktarma vakaları · Elle: kendi CSV dosyanla içe aktarma.
- **Ajan:** Sonnet (ayrıştırıcı, ardından ekran — ayrı görevler) · Haiku (tr.json) · Opus (kodlama algılama ve DB yazımını denetler).

### [ ] M1.10 · M1 kabul — v0.1.0

- **Hedef:** §13 M1 ölçütü + §14 ilgili elle testler; CHANGELOG; `v0.1.0` etiketi. Kurum içi yazılımda SendInput/pano denemesi [A] (§3.4).
- **Test:** Elle: Notepad, Word, Chrome, Outlook, kurum yazılımı; uyku/uyanma; boşta işlemci ~%0.
- **Ajan:** Saul (elle) · Opus (sonuçlara göre düzeltme).

---

## M2 · Maskot ve ada → v0.2.0

Bitti ölçütü: Maskotla arayıp yapıştırma çalışıyor.

### [ ] M2.1 · Tasarım tokenları ve i18n — v0.1.1

- **Hedef:** §11.4 renk rolleri CSS değişkeni olarak (şimdilik sabit açık/koyu varsayılan palet; üretici M6.1); 4 px ızgara, köşe, yay değerleri (380/28); küçük, bağımlılıksız `t()` yardımcı fonksiyonu.
- **Modüller:** `src/design/`, `src/i18n/`
- **Bitti:** Bileşenlerde sabit renk yok (lint kuralı / grep).
- **Test:** Oto: `t()` eksik anahtar testi · Elle: açık/koyu görünüm.
- **Ajan:** Sonnet.

### [ ] M2.2 · Maskot bileşeni — v0.1.2

- **Hedef:** `meetin-maskot.svg` yapısı ve sınıf adları aynen; `meetin-maskot-animasyon.html`'deki §5.1.1 animasyonları (`idle`, `graze`, `baa`, `stare`, `sleep`, `eat`, `hop`, `sad`, `squeeze`) taşınır; durum makinesi, göz bebeği takibi (tek `translate`), "kendi hâlinde" modu, `prefers-reduced-motion`, gizliyken animasyon durdurma; mee sesi (varsayılan kapalı).
- **Modüller:** `src/mascot/`
- **Bitti:** Referans HTML ile yan yana görsel olarak aynı; durum geçişleri testli.
- **Test:** Oto: durum makinesi · Elle: referans dosya ile yan yana karşılaştırma, 70/48 px okunurluk.
- **Ajan:** Sonnet · Opus denetim.

### [ ] M2.3 · Maskot penceresi — v0.1.3

- **Hedef:** Şeffaf, çerçevesiz, her zaman üstte, `WS_EX_NOACTIVATE`; sürükleme ve konum hatırlama (çoklu monitör, DPI); tek tık / çift tık / basılı tut / sağ tık ayrımı; sağ tık menüsü; göster/gizle kısayolu; tam ekranda kendiliğinden gizlenme; duraklatınca `sleep`.
- **Modüller:** `windows/`, `src/mascot/`
- **Bitti:** Maskot odak çalmadan her programın üstünde; tam ekran videoda görünmüyor.
- **Test:** Elle: iki monitör %100/%150, tam ekran video/sunum, uyku/uyanma; boşta işlemci ~%0.
- **Ajan:** Opus.

### [ ] M2.4 · Ada morph + arama adası — v0.1.4

- **Hedef:** Maskot → hap → panel morph (açılış ~350 ms, kapanış ~220 ms); `fuse.js` ile bulanık arama, ↑↓ gezinme, sağda önizleme, Esc; Enter → önceki etkin pencereye odak geri verilip yapıştırma.
- **Modüller:** `src/island/`, `windows/`, `inject/`
- **Bitti:** Arama → Enter metni doğru pencereye yazıyor; 10.000 ifadede arama < 30 ms.
- **Test:** Oto: arama benchmark · Elle: Notepad, Chrome, Word'de arayıp yapıştırma; yalnız klavyeyle kullanım.
- **Ajan:** Sonnet (ada arayüzü) · Opus (odak geri verme + yapıştırma komutu).

### [ ] M2.5 · Onay hapı ve olay animasyonları — v0.1.5

- **Hedef:** Rust → arayüz olayları: genişletme → `hop`, hata → `sad`; 1,2 sn onay hapı.
- **Modüller:** `src/island/`, `src/mascot/`, olay köprüsü
- **Bitti:** Her genişletmede doğru animasyon ve hap.
- **Test:** Elle: genişletme, olmayan kısaltma ile arama.
- **Ajan:** Sonnet.

### [ ] M2.6 · Yönetici penceresi rozeti — v0.1.6

- **Hedef:** Etkin pencere yükseltilmiş yetkideyse kalkan rozeti + ipucu (§3.6).
- **Modüller:** `windows/`, `src/mascot/`
- **Test:** Elle: yönetici olarak açılmış Notepad.
- **Ajan:** Opus.

### [ ] M2.7 · M2 kabul — v0.2.0

- **Test:** Elle: §13 M2 ölçütü, bellek < 120 MB, boşta işlemci. **Ajan:** Saul · Opus.

---

## M3 · Ekleme → v0.3.0

Bitti ölçütü: Üç ekleme yolu Word, Chrome ve kurum yazılımında çalışıyor.

### [ ] M3.1 · Seçili metni okuma — v0.2.1

- **Hedef:** UI Automation `TextPattern`; desteklenmiyorsa pano yedekle → `Ctrl+C` → oku → geri yükle.
- **Modüller:** `selection/`
- **Test:** Elle: Word, Chrome, Notepad, kurum yazılımı; pano içeriği korunuyor.
- **Ajan:** Opus.

### [ ] M3.2 · Seçim algılama + balon — v0.2.2

- **Hedef:** Fare kancasından sürükleme / çift / üç tık seçimi; ~300 ms sonra "+ Ekle" balonu; 3 sn veya yazmaya devamda kaybolma; program bazlı gizleme.
- **Modüller:** `hook/`, `windows/`, `src/bubble/`
- **Test:** Elle: farklı programlarda seçim; Photoshop/oyunda balon çıkmıyor.
- **Ajan:** Opus (algılama, pencere) · Sonnet (balon arayüzü).

### [ ] M3.3 · Ekleme adası — v0.2.3

- **Hedef:** Önizleme, baş harf kısaltma önerisi (tr-TR), anında çakışma uyarısı, açıklama, klasör, etiket; Enter kaydet → `eat`; "Ayrıntılı düzenle".
- **Modüller:** `src/island/`, `db/`
- **Test:** Oto: kısaltma önerisi ("Claude ile Proje Yapmak" → `CİPY`) · Elle: ekle → hemen genişlet.
- **Ajan:** Sonnet · Opus denetim.

### [ ] M3.4 · Kısayol, panodan ekleme, sıkma — v0.2.4

- **Hedef:** Seç + kısayol; "Panodakini ifade yap"; basılı tut ≥ 450 ms sıkma (1,26 × 0,72) → bırakınca ekleme.
- **Modüller:** `windows/`, `src/mascot/`, `src/island/`
- **Test:** Elle: üç yol.
- **Ajan:** Sonnet (sıkma, düğme) · Opus (kısayol kaydı).

### [ ] M3.5 · M3 kabul — v0.3.0 · **Ajan:** Saul · Opus.

---

## M4 · Zengin içerik → v0.4.0

Bitti ölçütü: Biçimli ifade Word/Outlook'a biçimiyle gidiyor.

### [ ] M4.1 · Değişken çözümleyici — v0.3.1

- **Hedef:** §3.3 sözdizimi: `{tarih}` (biçimler, ±g/h/a/y), `{saat}`, `{pano}`, `{imleç}`, `{alan:…}`, `{ifade:…}` + döngü koruması; Türkçe ay/gün adları.
- **Modüller:** `vars/`
- **Test:** Oto: her değişken, ay sonu/artık yıl, döngü.
- **Ajan:** Opus.

### [ ] M4.2 · Biçimli metin yapıştırma — v0.3.2

- **Hedef:** Saklama biçimi HTML; panoya CF_HTML + RTF + düz metin; biçim kabul etmeyen programa düz metin.
- **Modüller:** `inject/`, `db/`
- **Test:** Oto: HTML→RTF/CF_HTML dönüşümü · Elle: Word, Outlook, Notepad, Chrome.
- **Ajan:** Opus.

### [ ] M4.3 · `{imleç}` ve geri al — v0.3.3

- **Hedef:** Genişletme sonrası `←`; genişletmeden hemen sonra Backspace → kısaltmaya dönüş.
- **Modüller:** `inject/`, `hook/`
- **Test:** Oto: olay dizisi · Elle: Notepad, Word, Chrome.
- **Ajan:** Opus.

### [ ] M4.4 · Alan doldurma adası — v0.3.4

- **Hedef:** Kompakt form, metin kutusu ve açılır liste, Tab/Enter/Esc.
- **Modüller:** `src/island/`
- **Test:** Elle: iki alanlı ifade.
- **Ajan:** Sonnet.

### [ ] M4.5 · Program kuralları ve yapıştırma yöntemi — v0.3.5

- **Hedef:** §3.5 sadece/hariç kuralları (işlem adı + başlık kelimesi) motorda; program bazlı yapıştırma tuşu (`Ctrl+Shift+V`).
- **Modüller:** `matcher/`, `inject/`, `db/`
- **Test:** Oto: kural seçimi · Elle: Windows Terminal, tarayıcıda "WhatsApp" başlığı.
- **Ajan:** Opus.

### [ ] M4.6 · M4 kabul — v0.4.0 · **Ajan:** Saul · Opus.

---

## M5 · Uzman arayüzü → v0.5.0

Bitti ölçütü: PhraseExpress'teki temel işler yapılabiliyor.

### [ ] M5.1 · Düzen ve klasör ağacı — v0.4.1

- **Hedef:** Üç panel, Mica pencere, klasör ağacı (sürükle-bırak, renk, iç içe), sabit girişler, liste (sıralama/filtre), `Ctrl+K` komut paleti. M1.9 ekranı buraya taşınır.
- **Ajan:** Sonnet · Opus denetim. **Test:** Elle: klavye ve fare ile gezinme.

### [ ] M5.2 · Düzenleyici — v0.4.2

- **Hedef:** `@tiptap/react` + değişken hapları, etiket hapları, harf uyumu modu, ifade başına kısayol tuşu.
- **Ajan:** Sonnet (düzenleyici) · Opus (kısayol kaydı). **Test:** Oto: HTML ↔ hap dönüşümü · Elle.

### [ ] M5.3 · Kurallar, geçmiş, çöp kutusu — v0.4.3

- **Hedef:** Kural arayüzü; `phrase_versions` ile tek tık geri dönüş; çöp kutusu + ayarlanabilir kalıcı silme.
- **Ajan:** Sonnet (arayüz) · Opus (DB). **Test:** Oto: sürüm geri alma, temizleyici · Elle.

### [ ] M5.4 · Pano geçmişi — v0.4.4

- **Hedef:** Son 100 metin (ayarlı); gizli/parola yöneticisi içeriği alınmaz; kendi ara pano yazımlarımız alınmaz; tek tıkla ifadeye çevir.
- **Ajan:** Opus. **Test:** Oto: filtre · Elle: KeePass/Bitwarden kopyalama.

### [ ] M5.5 · İstatistik — v0.4.5

- **Hedef:** İlk 20, günlük/haftalık sayılar, kazanılan tuş ve dakika, maskot haftalık kartı.
- **Ajan:** Sonnet · Opus (sorgular). **Test:** Oto: hesaplama · Elle.

### [ ] M5.6 · İçe aktarma (kalanlar) + hızlı toplu ekleme — v0.4.6 … v0.4.8

- **Hedef:** xlsx, docx/txt, AHK, Beeftext JSON, Espanso YAML, PhraseExpress [A: örnek dosya]; ortak önizleme/çakışma ekranı; hızlı toplu ekleme ekranı. Her ayrıştırıcı ayrı commit.
- **Ajan:** Sonnet (ayrıştırıcı başına bir görev, sırayla) · Opus denetim. **Test:** Oto: her biçim için örnek dosyalar · Elle.

### [ ] M5.7 · Dışa aktarma — v0.4.9

- **Hedef:** `.meetin` (ZIP + JSON), CSV, AHK; `.meetin` → içe aktarma gidiş-dönüş.
- **Ajan:** Sonnet. **Test:** Oto: gidiş-dönüş eşitliği.

### [ ] M5.8 · M5 kabul — v0.5.0 · **Ajan:** Saul · Opus.

---

## M6 · Dağıtım → v1.0.0

Bitti ölçütü: Temiz bir Windows'ta kurulum → güncelleme → kaldırma sorunsuz.

### [ ] M6.1 · Palet üretici — v0.5.1

- **Hedef:** Saf TS, OKLCH; 11 basamak, nötrler, vurgu üstü metin, durum renkleri, koyu tema, 4,5:1 güvencesi; elle değişen rolleri koruma.
- **Ajan:** Opus. **Test:** Oto: hazır renkler + rastgele 500 renk, açık/koyu, tüm çiftler ≥ 4,5:1.

### [ ] M6.2 · Windows tema uyumu — v0.5.2

- **Hedef:** Açık/koyu ve vurgu rengi canlı izleme, Mica/Akrilik (`window-vibrancy`), saydamlık kapalı, `forced-colors`, Windows 10 yedek görünüm.
- **Ajan:** Opus. **Test:** Elle: §14 tema maddeleri.

### [ ] M6.3 · Görünüm ayarları — v0.5.3

- **Hedef:** §11.2 üç düzey, canlı önizleme, karşıtlık rozetleri, ada yüzeyi, saydamlık, `.meetin-tema` içe/dışa, varsayılana dön.
- **Ajan:** Sonnet · Opus denetim. **Test:** Elle.

### [ ] M6.4 · Diğer ayar sayfaları — v0.5.4

- **Hedef:** Genel, Tetikleme, Harf uyumu, Maskot, Kısayollar (çakışma uyarısı), Dışlanan programlar, Dil.
- **Ajan:** Sonnet · Haiku (tr.json). **Test:** Elle: her ayar gerçekten etki ediyor.

### [ ] M6.5 · Yedek arayüzü — v0.5.5

- **Hedef:** "Şimdi yedekle", "Yedekten geri yükle" (geri yüklemeden önce mevcut DB'nin yedeği).
- **Ajan:** Opus. **Test:** Oto: geri yükleme · Elle.

### [ ] M6.6 · İlk açılış — v0.5.6

- **Hedef:** `sleep` → `hop`, isim sorma, 3 adımlık tanıtım.
- **Ajan:** Sonnet. **Test:** Elle: temiz veri klasörüyle.

### [ ] M6.7 · Yazarken öneri adası — v0.5.7

- **Hedef:** 2+ harfte 1–3 öneri hapı, `Tab` ile seçim; parola kutusu ve dışlanan programlarda kapalı.
- **Ajan:** Opus (kanca tarafı) · Sonnet (haplar). **Test:** Elle: dikkat dağıtıyor mu [A].

### [ ] M6.8 · Başlangıç ve yönetici modu — v0.5.8

- **Hedef:** Windows ile başla; "Yönetici olarak başlat" (Görev Zamanlayıcı, UAC'siz).
- **Ajan:** Opus. **Test:** Elle: yeniden başlatma.

### [ ] M6.9 · Paketleme ve güncelleme — v0.5.9

- **Hedef:** NSIS kurulum, portable ZIP, imzalı `latest.json` + GitHub Releases, portable kendini güncelleme (`Meetin.old.exe`), WebView2 kontrolü, maskot güncelleme rozeti, "Şimdi kur / Sonra".
- **Ajan:** Opus. **Test:** Elle: sahte bir 0.5.10 sürümüyle güncelleme denemesi.

### [ ] M6.10 · M6 kabul — v1.0.0

- **Test:** Elle: temiz Windows 10 ve 11 (VM) kurulum → güncelleme → kaldırma; 24 saat açık kalma, bellek; salt okunur USB'de portable. **Ajan:** Saul · Opus.

---

### Not

- **M1.4 (otomatik yedek)** şartnamede M6'da; veri kaybı riski M1'de başladığı için öne alındı. Geri yükleme arayüzü yine M6.5'te.
