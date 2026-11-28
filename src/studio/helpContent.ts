import { getLanguage, type Language } from "../i18n";

export type HelpSectionId =
  | "quickstart"
  | "turkish-case"
  | "variables"
  | "library"
  | "shortcuts"
  | "clipboard"
  | "mascot"
  | "backup-io"
  | "appearance"
  | "faq";

export type HelpBlock =
  | { type: "paragraph"; text: string }
  | { type: "steps"; title?: string; items: string[] }
  | { type: "bullets"; title?: string; items: { label: string; text: string }[] }
  | {
      type: "variables";
      title?: string;
      items: { code: string; example: string; desc: string }[];
    }
  | { type: "table"; title?: string; headers: string[]; rows: string[][] }
  | { type: "tip"; tone: "info" | "warn"; text: string }
  | { type: "faq"; items: { q: string; a: string }[] };

export interface HelpSection {
  id: HelpSectionId;
  title: string;
  summary: string;
  keywords: string[];
  blocks: HelpBlock[];
}

export const HELP_SECTIONS: HelpSection[] = [
  {
    id: "quickstart",
    title: "Hızlı Başlangıç",
    summary:
      "İlk kısaltmanızı saniyeler içinde oluşturun, herhangi bir programda yazarak anında genişletin.",
    keywords: [
      "başlangıç",
      "ilk ifade",
      "tetikleyici",
      "boşluk",
      "enter",
      "tab",
      "noktalama",
      "otomatik kaydetme",
      "ekle balonu",
      "ctrl+alt+e",
    ],
    blocks: [
      {
        type: "paragraph",
        text: "Meetin arka planda sessizce çalışan akıllı bir Türkçe metin genişleticidir. Bir kez tanımladığınız kısaltmayı Word, Excel, tarayıcı, e-posta istemcisi veya not defteri gibi tüm programlarda yazdığınız anda tam metne dönüştürür.",
      },
      {
        type: "steps",
        title: "İlk ifadenizi oluşturma",
        items: [
          "Orta sütundaki \"+ Yeni İfade\" düğmesine tıklayın (veya Ctrl+Alt+E kısayolu ile her yerden hızlı ekleme adasını açın).",
          "İçerik alanına genişletilecek tam metni yazın; siz düzenleme yaparken açıklama alanı ilk satırdan otomatik doldurulur (isterseniz elle değiştirebilirsiniz).",
          "Kısaltma alanına yazacağınız tetikleyici kodu girin (ör. SYG). Meetin değişikliklerinizi 650 ms içinde otomatik kaydeder.",
          "Herhangi bir uygulamaya geçip kısaltmayı yazın ve ardından bir tetikleyici tuşa (Boşluk, Enter, Tab veya noktalama) basın.",
        ],
      },
      {
        type: "bullets",
        title: "Diğer hızlı ekleme yolları",
        items: [
          {
            label: "+ Ekle seçim balonu",
            text: "Herhangi bir uygulamada metin seçtiğinizde beliren küçük balondan seçili metni tek tıkla yeni ifade taslağına dönüştürebilirsiniz.",
          },
          {
            label: "Tetikleyici tuşlar",
            text: "Boşluk, Enter, Tab ve noktalama işaretlerinin hangilerinin genişletmeyi tetikleyeceğini Ayarlar › Tetikleme bölümünden belirleyebilirsiniz.",
          },
        ],
      },
      {
        type: "tip",
        tone: "info",
        text: "İpucu: İçerik alanını doldurduktan sonra kısaltma önerisi otomatik üretilir. Ayarlar › Genel altından öneri kuralını (İlk harfler, İlk kelime, Son kelime, Etiket) kendinize göre seçebilirsiniz.",
      },
    ],
  },
  {
    id: "turkish-case",
    title: "Türkçe Harf Uyumu ve Shift+F3",
    summary:
      "i ↔ İ ve ı ↔ I dönüşümlerini kusursuz koruyan genişletme modları ve art arda çalışan Shift+F3 harf döngüsü.",
    keywords: [
      "türkçe",
      "harf uyumu",
      "büyük küçük",
      "aynen getir",
      "birebir",
      "aksanları yoksay",
      "noktalı i",
      "shift+f3",
      "döngü",
    ],
    blocks: [
      {
        type: "paragraph",
        text: "Meetin Türkçe alfabe kurallarına (tr-TR) göre tasarlanmıştır; küçük 'i' her zaman büyük 'İ', küçük 'ı' ise her zaman büyük 'I' olur. Her ifade için üç farklı Büyük/Küçük Harf Uyumu modu seçebilirsiniz:",
      },
      {
        type: "table",
        title: "Büyük/Küçük Harf Uyumu modları",
        headers: ["Simge", "Mod Adı", "Davranış ve Örnek"],
        rows: [
          [
            "A=",
            "Aynen getir (Varsayılan)",
            "Kısaltmayı küçük ya da büyük nasıl yazarsanız yazın, içeriğin kayıtlı büyük/küçük harf biçimi birebir korunur.",
          ],
          [
            "Aa",
            "Harf uyumu",
            "Yazdığınız kısaltmanın harf biçimine uyar: syg → saygılarımla, Syg → Saygılarımla, SYG → SAYGILARIMLA.",
          ],
          [
            "AA",
            "Birebir eşleşme",
            "Yalnızca kısaltma kayıtlı olduğu büyük/küçük harf biçimiyle birebir yazıldığında genişler.",
          ],
        ],
      },
      {
        type: "bullets",
        title: "Aksanlar ve Shift+F3",
        items: [
          {
            label: "Aksanları yoksay",
            text: "Ayarlar › Harf uyumu altında etkinleştirildiğinde aksanlı ve noktalı/noktasız harfler eşleştirme sırasında esnek kabul edilir.",
          },
          {
            label: "Shift+F3 Harf Döngüsü",
            text: "Hangi düzenlenebilir uygulamada olursanız olun bir metni seçip Shift+F3 tuşlarına basın. Metin Türkçe kurallarıyla tümü küçük → İlk harf büyük → TÜMÜ BÜYÜK arasında döner. Dönüşüm sonrası seçim korunur; böylece elinizi kaldırmadan art arda Shift+F3 basarak istediğiniz biçime geçebilirsiniz.",
          },
        ],
      },
    ],
  },
  {
    id: "variables",
    title: "Değişkenler ve Form Alanları",
    summary:
      "Dinamik tarih, saat, pano, imleç konumu, açılır seçim kutuları ve iç içe ifadeler ekleyin.",
    keywords: [
      "değişken",
      "tarih",
      "saat",
      "pano",
      "imleç",
      "alan",
      "form",
      "ifade",
      "biçimli",
      "html",
      "bağlantı",
    ],
    blocks: [
      {
        type: "paragraph",
        text: "İçerik düzenleyicideki \"+ Değişken ekle\" menüsünden veya süslü parantezlerle aşağıdaki dinamik yer tutucuları doğrudan yazabilirsiniz:",
      },
      {
        type: "variables",
        title: "Desteklenen değişken söz dizimi",
        items: [
          {
            code: "{tarih}",
            example: "26.11.2026",
            desc: "Günün tarihini GG.AA.YYYY biçiminde ekler.",
          },
          {
            code: "{tarih:uzun}",
            example: "26 Kasım 2026 Perşembe",
            desc: "Ay ve gün adıyla tam Türkçe tarih ekler.",
          },
          {
            code: "{tarih:+1g}",
            example: "+1g, -2h, +1a, -1y",
            desc: "Tarihi gün (g), hafta (h), ay (a) veya yıl (y) bazında ileri/geri kaydırır.",
          },
          {
            code: "{saat}",
            example: "18:05",
            desc: "Güncel saati 24 saat biçiminde ekler.",
          },
          {
            code: "{pano}",
            example: "(Panodaki son metin)",
            desc: "Windows panosundaki güncel metni şablonun içine yerleştirir.",
          },
          {
            code: "{imleç}",
            example: "Merhaba {imleç},",
            desc: "Genişletme tamamlandığında metin imlecini tam bu noktaya taşır.",
          },
          {
            code: "{alan:İsim}",
            example: "Sayın {alan:İsim},",
            desc: "Genişletme anında küçük bir pencere açarak sizden bilgi girmeyi ister.",
          },
          {
            code: "{alan:Durum|Acil,Normal,Düşük}",
            example: "Seçenekli liste",
            desc: "Çubuk (|) sonrası virgülle ayrılan hazır seçeneklerden açılır liste sunar.",
          },
          {
            code: "{ifade:KISALTMA}",
            example: "{ifade:SYG}",
            desc: "Başka bir kayıtlı ifadeyi bu şablonun içine gömer (döngü korumalıdır).",
          },
        ],
      },
      {
        type: "tip",
        tone: "info",
        text: "İçerik üst çubuğundaki B (Kalın), I (İtalik), U (Altı çizili) ve bağlantı sembolleriyle biçimli metin oluşturabilirsiniz; Word ve e-posta editörlerine HTML/RTF olarak aktarılır.",
      },
    ],
  },
  {
    id: "library",
    title: "Klasörler, Toplu İşlem ve Düzenleme",
    summary:
      "Çoklu klasörler, sürükle-bırak taşıma/kopyalama, klasör renkleri, sola kaydırıp silme, toplu seçme ve uygulama kuralları.",
    keywords: [
      "klasör",
      "renk",
      "etiket",
      "sürükle",
      "kopyala",
      "toplu",
      "seçim",
      "sağ tık",
      "kaydır",
      "çöp kutusu",
      "sürüm geçmişi",
      "uygulama kuralı",
      "daralt",
    ],
    blocks: [
      {
        type: "bullets",
        title: "Klasörler, renkler ve etiketler",
        items: [
          {
            label: "Çoklu klasör üyeliği",
            text: "Aynı ifade birden fazla klasörde yer alabilir; ancak karışıklığı önlemek için aynı kısaltma ikinci kez oluşturulamaz.",
          },
          {
            label: "Sürükle-bırak taşıma ve Ctrl+kopyala",
            text: "Bir veya birden çok seçili ifadeyi sol menüdeki bir klasörün üzerine sürükleyerek taşıyın. Sürüklerken Ctrl tuşunu basılı tutarsanız kopyası ilgili klasöre eklenir.",
          },
          {
            label: "Otomatik klasör etiketi",
            text: "Yeni ifade oluştururken veya bir ifadeye klasör seçtiğinizde klasörün adı etiketlere otomatik eklenir; etiketleri istediğiniz gibi düzenleyip silebilirsiniz.",
          },
          {
            label: "Klasör renkleri ve sağ tık menüsü",
            text: "Sol menüdeki herhangi bir klasöre sağ tıklayarak 8 renkten birini atayabilir, klasörü yeniden adlandırabilir, yalnızca o klasörü dışa aktarabilir veya silebilirsiniz.",
          },
          {
            label: "Daraltılabilir sol menü",
            text: "Sol üstteki « / » düğmesiyle yan menüyü simge moduna küçültebilirsiniz.",
          },
        ],
      },
      {
        type: "bullets",
        title: "Toplu işlem, silme ve kurallar",
        items: [
          {
            label: "Toplu seçme",
            text: "Ctrl+tıkla tek tek, Shift+tıkla aralık, Ctrl+A ile tümünü seçin; sağ tıklayınca açılan menüden seçilenlerin hepsini taşıyabilir, kopyalayabilir, dışa aktarabilir veya silebilirsiniz. Esc seçimi temizler.",
          },
          {
            label: "Sola kaydırarak veya Çöp Kutusuna atarak silme",
            text: "Bir ifadeyi sol menüdeki Çöp Kutusu üzerine sürükleyebilir, satırı dokunmatik yüzeyle sola kaydırıp beliren kırmızı Sil düğmesine tıklayabilir ya da sağ tık menüsünden Sil'i seçebilirsiniz.",
          },
          {
            label: "Uygulama kuralları ve Sürüm Geçmişi",
            text: "Sağ panelden ifadenin yalnızca belirli uygulamalarda çalışmasını veya hariç tutulmasını açık pencereler listesinden seçebilirsiniz. Değişiklik geçmişinden eski bir sürüme tek tıkla dönebilirsiniz.",
          },
        ],
      },
    ],
  },
  {
    id: "shortcuts",
    title: "Kısayollar ve Komut Paleti",
    summary:
      "Meetin'i klavyeden yönetmek için global kısayollar ve ana pencere komut paleti.",
    keywords: [
      "kısayol",
      "komut paleti",
      "ctrl+k",
      "f1",
      "shift+f3",
      "ctrl+alt+f",
      "ctrl+alt+e",
      "ctrl+alt+s",
      "ctrl+alt+p",
      "ctrl+alt+space",
    ],
    blocks: [
      {
        type: "paragraph",
        text: "Global kısayolları Ayarlar › Kısayollar bölümünden kendi alışkanlıklarınıza göre değiştirebilirsiniz. Ana pencere açıkken Ctrl+K ile Komut Paletini, F1 ile bu Yardım Merkezini anında açabilirsiniz.",
      },
    ],
  },
  {
    id: "clipboard",
    title: "Pano Geçmişi",
    summary:
      "Kopyaladığınız son metinleri saklayın, harf biçimini değiştirerek kopyalayın veya ifadeye dönüştürün.",
    keywords: [
      "pano",
      "geçmiş",
      "kopyala",
      "büyük harf",
      "küçük harf",
      "ilk harf büyük",
      "10 kopya",
    ],
    blocks: [
      {
        type: "bullets",
        title: "Pano Geçmişi özellikleri",
        items: [
          {
            label: "Ayarlanabilir kayıt limiti",
            text: "Varsayılan olarak kopyaladığınız son 10 metin yerel veritabanında tutulur. Bu limiti Ayarlar › Genel üzerinden açıp kapatabilir veya değiştirebilirsiniz.",
          },
          {
            label: "Biçim dönüştürerek yapıştırma/kopyalama",
            text: "Sol menüden Pano Geçmişi sekmesine geçerek her kaydın yanındaki Kopyala, TÜMÜ BÜYÜK, tümü küçük veya İlk Harf Büyük düğmelerini tek tıkla kullanabilirsiniz.",
          },
          {
            label: "İfadeye dönüştürme",
            text: "Herhangi bir pano kaydını \"+ İfade Yap\" düğmesiyle anında yeni bir kısaltma taslağına aktarabilirsiniz.",
          },
        ],
      },
    ],
  },
  {
    id: "mascot",
    title: "Maskot ve Ses Kontrolleri",
    summary:
      "Masaüstü koyun maskotuyla etkileşim kurun, arama adasını açın, basket atın ve her sesi ayrı ayrı yönetin.",
    keywords: [
      "maskot",
      "koyun",
      "ada",
      "arama",
      "sessize al",
      "ses",
      "mee",
      "fırlat",
      "basket",
      "boyut",
    ],
    blocks: [
      {
        type: "bullets",
        title: "Maskot etkileşimleri",
        items: [
          {
            label: "Sol tık / Çift tık",
            text: "Maskota tek tıklamak hızlı arama adasını açar; burada arayıp Enter ile doğrudan odaklandığınız uygulamaya yapıştırabilirsiniz. Çift tıklamak ana pencereyi öne getirir.",
          },
          {
            label: "Sürükle, fırlat ve Basketbol oyunu",
            text: "Maskotu masaüstünde dilediğiniz köşeye sürükleyebilir veya hızla savurup duvardan sektirebilirsiniz. Sağ tık menüsünden Mini Oyun: Basketbol açıp potaya basket atmayı deneyebilirsiniz.",
          },
          {
            label: "Sağ tıkla hızlı sessize alma",
            text: "Maskota sağ tıklayarak doğrudan Sessize Al / Sesi Aç, Meetin'i Aç, Gizle ve Çıkış işlemlerine ulaşabilirsiniz.",
          },
          {
            label: "Ayrı ses kontrolleri",
            text: "Ayarlar › Maskot altında ana ses anahtarına ek olarak Mee sesi, Fırlatma sesi ve Duvar çarpma sesi ayrı ayrı açılıp kapatılabilir ve test edilebilir.",
          },
        ],
      },
    ],
  },
  {
    id: "backup-io",
    title: "İçe / Dışa Aktarma ve Yedekleme",
    summary:
      "Kütüphanenizi .mee, CSV ve AutoHotkey biçimlerinde paylaşın; otomatik AppData yedekleriyle güvende tutun.",
    keywords: [
      "yedek",
      "dışa aktar",
      "içe aktar",
      "mee",
      "meetin",
      "csv",
      "ahk",
      "beeftext",
      "appdata",
      "portable",
      "geri yükle",
    ],
    blocks: [
      {
        type: "bullets",
        title: "Aktarma ve veri konumu",
        items: [
          {
            label: "Esnek dışa aktarma (.mee / CSV / .ahk)",
            text: "Sağ üstteki ⋯ menüsünden tüm kütüphaneyi, orta liste başlığından o anda görünen/filtrelenen listeyi, sağ tık menüsünden seçili klasör veya seçili ifadeleri dışa aktarabilirsiniz.",
          },
          {
            label: "Çoklu içe aktarma",
            text: ".mee, .meetin, CSV (UTF-8 ve Windows-1254 Türkçe algılamalı), .ahk ve .txt dosyalarını çakışma stratejisi seçerek (Atla / Üzerine yaz / Yeni kısaltma öner) içe aktarabilirsiniz.",
          },
          {
            label: "Veri klasörü ve otomatik yedek",
            text: "Tüm veriler %APPDATA%\\Meetin\\ altında saklanır. Açılışta ve kapanışta otomatik SQLite yedeği alınır (son 30 yedek saklanır). Taşınabilir sürümde EXE yanına boş local-data.flag dosyası koyarak yerel ./data/ klasörünü kullanabilirsiniz.",
          },
        ],
      },
    ],
  },
  {
    id: "appearance",
    title: "Görünüm, Tema ve Yazı Boyutu",
    summary:
      "Windows açık/koyu modu, WCAG kontrast güvenceli OKLCH vurgu renkleri ve ayarlanabilir yazı boyutu.",
    keywords: [
      "görünüm",
      "tema",
      "açık",
      "koyu",
      "renk",
      "oklch",
      "yazı boyutu",
      "akrilik",
      "mee-tema",
    ],
    blocks: [
      {
        type: "bullets",
        title: "Kişiselleştirme seçenekleri",
        items: [
          {
            label: "Uygulama modu ve sistem uyumu",
            text: "Ayarlar › Görünüm bölümünde Sistem, Açık veya Koyu modu seçebilir, Windows vurgu rengini otomatik takip edebilirsiniz.",
          },
          {
            label: "Özel ana renk ve canlı önizleme",
            text: "\"Ana renk seç\" ile OKLCH tabanlı paletten bir ton belirleyin; metin karşıtlığı okunabilirlik (WCAG ≥ 4,5:1) için otomatik dengelenir. Sağ panelden canlı önizlemeyi anlık görebilir, temayı .mee-tema olarak dışa aktarabilirsiniz.",
          },
          {
            label: "Yazı boyutu",
            text: "Küçük (12px), Normal (14px), Büyük (16px) ve Çok Büyük (18px) seçenekleriyle tüm arayüz yazı boyutunu tek tıkla ölçekleyebilirsiniz.",
          },
        ],
      },
    ],
  },
  {
    id: "faq",
    title: "Sık Sorulan Sorular",
    summary:
      "Çalışmayan kısaltmalar, yönetici yetkisi, parola alanı güvenliği ve Windows SmartScreen hakkında yanıtlar.",
    keywords: [
      "sss",
      "sorun",
      "çalışmıyor",
      "yönetici",
      "admin",
      "parola",
      "güvenlik",
      "smartscreen",
      "güncelleme",
    ],
    blocks: [
      {
        type: "faq",
        items: [
          {
            q: "Kısaltmayı yazıyorum ama genişlemiyor, neden?",
            a: "Üst çubukta durumun 'Etkin' (duraklatılmamış) olduğunu doğrulayın. Hedef program Ayarlar › Dışlanan programlar listesinde olabilir veya seçili ifadenin yalnızca başka uygulamalara özel bir kuralı bulunabilir. Ayrıca Yönetici olarak açılmış pencerelere (ör. Görev Yöneticisi) yazabilmek için Ayarlar › Genel içinden 'Yönetici yetkili başlangıç' seçeneğini etkinleştirebilirsiniz.",
          },
          {
            q: "Shift+F3 neden bazı alanlarda harf değiştirmiyor?",
            a: "Shift+F3 yalnızca metin seçilebilen ve düzenlenebilen (yazılabilir) alanlarda çalışır. Salt okunur web sayfalarında, PDF görüntüleyicilerde veya güvenli parola kutularında işletim sistemi gereği değişiklik yapılmaz.",
          },
          {
            q: "Meetin parola alanlarında yazdıklarımı okur mu?",
            a: "Hayır. Windows UI Automation (IsPassword) tarafından parola alanı olarak işaretlenen kutular otomatik olarak atlanır; yazılan karakterler tampona alınmaz ve pano geçmişine kaydedilmez.",
          },
          {
            q: "Kurulumda Windows SmartScreen uyarısı çıkarsa ne yapmalıyım?",
            a: "Meetin açık kaynaklı bir yazılımdır ve ticari kod imzalama sertifikası olmadan dağıtıldığı için Windows Defender SmartScreen ilk açılışta onay isteyebilir. 'Ek bilgi' → 'Yine de çalıştır' adımıyla güvenle başlatabilirsiniz.",
          },
          {
            q: "Verilerimi başka bir bilgisayara nasıl taşırım?",
            a: "Sağ üst ⋯ menüsünden '.mee olarak dışa aktar' seçeneğini kullanabilir veya Ayarlar › Yedek bölümünden aldığınız son yedeği yeni bilgisayardaki Meetin penceresinden geri yükleyebilirsiniz.",
          },
        ],
      },
    ],
  },
];

export const HELP_SECTIONS_EN: HelpSection[] = [
  {
    id: "quickstart",
    title: "Quick Start",
    summary:
      "Create your first abbreviation in seconds and expand it instantly in any application.",
    keywords: [
      "start",
      "quickstart",
      "first phrase",
      "trigger",
      "space",
      "enter",
      "tab",
      "punctuation",
      "auto-save",
      "add bubble",
      "ctrl+alt+e",
    ],
    blocks: [
      {
        type: "paragraph",
        text: "Meetin is a smart desktop text expander that runs quietly in the background. Define an abbreviation once and expand it into full text across Word, Excel, browsers, email clients, or plain editors.",
      },
      {
        type: "steps",
        title: "Create your first phrase",
        items: [
          'Click "+ New Phrase" in the middle column (or press Ctrl+Alt+E anywhere to open the quick add island).',
          "Type or format your expanded text in the live WYSIWYG editor; the description auto-fills from the first line (editable anytime).",
          "Enter your short trigger abbreviation (e.g. BRG). Meetin auto-saves your changes within 650 ms.",
          "Switch to any application, type the abbreviation, and press a trigger key (Space, Enter, Tab, or punctuation).",
        ],
      },
      {
        type: "bullets",
        title: "Other quick ways to add phrases",
        items: [
          {
            label: "+ Add selection bubble",
            text: "Highlight text in any app and click the floating + Add bubble to turn it into a phrase in one click.",
          },
          {
            label: "Trigger keys",
            text: "Customize whether Space, Enter, Tab, or punctuation triggers expansion under Settings › Triggers.",
          },
        ],
      },
      {
        type: "tip",
        tone: "info",
        text: "Tip: Abbreviation suggestions are generated automatically. Pick your preferred rule (Initials, First word, Last word, Tag) under Settings › General.",
      },
    ],
  },
  {
    id: "turkish-case",
    title: "Turkish Case Matching & Shift+F3",
    summary:
      "Expansion case modes with accurate Turkish i ↔ İ / ı ↔ I handling and repeatable Shift+F3 case cycling.",
    keywords: [
      "turkish",
      "case",
      "uppercase",
      "lowercase",
      "keep original",
      "exact match",
      "accents",
      "dotted i",
      "shift+f3",
      "cycle",
    ],
    blocks: [
      {
        type: "paragraph",
        text: "Meetin handles Turkish locale casing accurately (lowercase 'i' maps to 'İ' and 'ı' maps to 'I'). Choose from three Case Matching modes per phrase:",
      },
      {
        type: "table",
        title: "Case Matching modes",
        headers: ["Symbol", "Mode", "Behavior & Example"],
        rows: [
          [
            "A=",
            "Keep original (Default)",
            "Preserves the saved casing of your phrase regardless of how you type the abbreviation.",
          ],
          [
            "Aa",
            "Match typed case",
            "Adapts to what you type: brg → best regards, Brg → Best regards, BRG → BEST REGARDS.",
          ],
          [
            "AA",
            "Exact match",
            "Expands only when typed with the exact saved uppercase/lowercase letters.",
          ],
        ],
      },
      {
        type: "bullets",
        title: "Accents and Shift+F3",
        items: [
          {
            label: "Ignore accents",
            text: "Enable this under Settings › Case Matching to match accented and ASCII characters flexibly.",
          },
          {
            label: "Shift+F3 Case Cycler",
            text: "Select text in any editable field and press Shift+F3 to cycle through lowercase → Sentence case → ALL CAPS while keeping your selection active.",
          },
        ],
      },
    ],
  },
  {
    id: "variables",
    title: "Variables, Forms & Rich Formatting",
    summary:
      "Insert dynamic dates, times, clipboard text, cursor placement, interactive form fields, and live WYSIWYG formatting.",
    keywords: [
      "variable",
      "date",
      "time",
      "clipboard",
      "cursor",
      "field",
      "form",
      "nested",
      "rich text",
      "wysiwyg",
      "html",
      "link",
      "list",
    ],
    blocks: [
      {
        type: "paragraph",
        text: 'Use the "+ Insert variable" menu in the editor toolbar or type curly-brace placeholders directly:',
      },
      {
        type: "variables",
        title: "Supported variable syntax",
        items: [
          {
            code: "{tarih}",
            example: "26.11.2026",
            desc: "Inserts today's date in DD.MM.YYYY format.",
          },
          {
            code: "{tarih:uzun}",
            example: "26 Kasım 2026 Perşembe",
            desc: "Inserts the full date with month and weekday name.",
          },
          {
            code: "{tarih:+1g}",
            example: "+1g, -2h, +1a, -1y",
            desc: "Offsets the date forward or backward by day (g), week (h), month (a), or year (y).",
          },
          {
            code: "{saat}",
            example: "18:05",
            desc: "Inserts the current time in 24-hour format.",
          },
          {
            code: "{pano}",
            example: "(Latest clipboard text)",
            desc: "Inserts the current Windows clipboard text into the template.",
          },
          {
            code: "{imleç}",
            example: "Hello {imleç},",
            desc: "Places the text cursor at this exact spot after expansion.",
          },
          {
            code: "{alan:Name}",
            example: "Dear {alan:Name},",
            desc: "Opens a quick fill island when expanding to prompt for input.",
          },
          {
            code: "{alan:Status|Urgent,Normal,Low}",
            example: "Dropdown list",
            desc: "Offers a dropdown list from comma-separated options after the pipe (|).",
          },
          {
            code: "{ifade:ABBR}",
            example: "{ifade:BRG}",
            desc: "Embeds another saved phrase inside this template (with cycle protection).",
          },
        ],
      },
      {
        type: "tip",
        tone: "info",
        text: "Use Bold, Italic, Underline, Bulleted/Numbered Lists, Links, and Embedded Images in the live WYSIWYG toolbar; formatted content pastes cleanly into Word and Outlook via CF_HTML and RTF.",
      },
    ],
  },
  {
    id: "library",
    title: "Folders, Bulk Actions & Rules",
    summary:
      "Multi-folder organization, drag-and-drop move/copy, folder colors, swipe delete, bulk selection, and app rules.",
    keywords: [
      "folder",
      "color",
      "tag",
      "drag",
      "copy",
      "bulk",
      "selection",
      "context menu",
      "swipe",
      "trash",
      "history",
      "app rule",
      "sidebar",
    ],
    blocks: [
      {
        type: "bullets",
        title: "Folders, colors, and tags",
        items: [
          {
            label: "Multi-folder membership",
            text: "A phrase can belong to multiple folders at once while keeping abbreviations unique.",
          },
          {
            label: "Drag-and-drop move & Ctrl+copy",
            text: "Drag one or more selected phrases onto any folder to move them; hold Ctrl while dropping to add to that folder as well.",
          },
          {
            label: "Folder colors & right-click menu",
            text: "Right-click any folder to pick from 8 colors, rename, export that folder alone, or delete it.",
          },
          {
            label: "Collapsible sidebar",
            text: "Click « / » at the top of the sidebar to switch between compact icon mode and full labels.",
          },
        ],
      },
      {
        type: "bullets",
        title: "Bulk actions, deletion, and rules",
        items: [
          {
            label: "Multi-select",
            text: "Ctrl+click, Shift+click, or Ctrl+A to select; right-click to move, copy, export, or delete all selected phrases. Esc clears the selection.",
          },
          {
            label: "App rules & version history",
            text: "Restrict a phrase to specific open apps or exclude apps from the detail panel, and restore any prior version in one click.",
          },
        ],
      },
    ],
  },
  {
    id: "shortcuts",
    title: "Shortcuts & Command Palette",
    summary:
      "Global keyboard shortcuts to control Meetin from anywhere plus the in-app Command Palette.",
    keywords: [
      "shortcut",
      "command palette",
      "ctrl+k",
      "f1",
      "shift+f3",
      "ctrl+alt+f",
      "ctrl+alt+e",
      "ctrl+alt+s",
      "ctrl+alt+p",
      "ctrl+alt+space",
    ],
    blocks: [
      {
        type: "paragraph",
        text: "Customize global shortcuts under Settings › Shortcuts. Inside the main window, press Ctrl+K to open the Command Palette and F1 to toggle this Help Center.",
      },
    ],
  },
  {
    id: "clipboard",
    title: "Clipboard History",
    summary:
      "Keep recent copied text snippets, transform their casing on copy, or turn them into phrases.",
    keywords: [
      "clipboard",
      "history",
      "copy",
      "uppercase",
      "lowercase",
      "title case",
      "recent copies",
    ],
    blocks: [
      {
        type: "bullets",
        title: "Clipboard History features",
        items: [
          {
            label: "Configurable history limit",
            text: "Stores your last 10 copied text snippets locally by default. Adjust or disable this under Settings › General.",
          },
          {
            label: "Case transform actions",
            text: "Open Clipboard History from the sidebar to copy any item as original, ALL CAPS, lowercase, or Title Case.",
          },
          {
            label: "Convert to phrase",
            text: 'Click "Convert to Phrase" on any clipboard item to prefill a new phrase draft immediately.',
          },
        ],
      },
    ],
  },
  {
    id: "mascot",
    title: "Mascot & Sound Controls",
    summary:
      "Interact with the desktop sheep mascot, launch the search island, play mini basketball, and configure sounds individually.",
    keywords: [
      "mascot",
      "sheep",
      "island",
      "search",
      "mute",
      "sound",
      "baa",
      "throw",
      "basketball",
      "size",
    ],
    blocks: [
      {
        type: "bullets",
        title: "Mascot interactions",
        items: [
          {
            label: "Click / Double-click",
            text: "Single-click opens the quick search island where Enter pastes directly into your active app. Double-click brings the main window to the front.",
          },
          {
            label: "Drag, throw & Basketball mini game",
            text: "Drag the mascot anywhere on screen or toss it against the edges. Open Mini Game: Basketball from its right-click menu.",
          },
          {
            label: "Individual sound toggles",
            text: "Under Settings › Mascot, toggle master sound or individually control and preview Baa, Throw, and Wall Impact sounds.",
          },
        ],
      },
    ],
  },
  {
    id: "backup-io",
    title: "Import / Export & Backups",
    summary:
      "Share libraries in .mee (rich ZIP), CSV, and AutoHotkey formats, protected by automatic SQLite backups.",
    keywords: [
      "backup",
      "export",
      "import",
      "mee",
      "meetin",
      "csv",
      "ahk",
      "appdata",
      "portable",
      "restore",
      "zip",
    ],
    blocks: [
      {
        type: "bullets",
        title: "Transfer & data location",
        items: [
          {
            label: "Flexible export (.mee / CSV / .ahk)",
            text: "Export your entire library from the top-right ⋯ menu, the current filtered list from the list header, or individual folders/phrases via right-click.",
          },
          {
            label: "Rich .mee archive import",
            text: "Import .mee rich ZIP archives (with embedded images), legacy JSON .mee/.meetin, CSV, .ahk, and .txt files with conflict handling (Skip / Overwrite / Suggest new).",
          },
          {
            label: "Automatic SQLite backups",
            text: "Data is stored under %APPDATA%\\Meetin\\ with automatic startup and shutdown backups (up to 30 kept). Portable builds with a local-data.flag file store data in ./data/.",
          },
        ],
      },
    ],
  },
  {
    id: "appearance",
    title: "Appearance, Theme & Language",
    summary:
      "Windows 11 Mica/Acrylic translucency, WCAG-checked OKLCH accent palettes, font scaling, and live language switching.",
    keywords: [
      "appearance",
      "theme",
      "light",
      "dark",
      "color",
      "oklch",
      "font size",
      "mica",
      "acrylic",
      "language",
      "english",
      "turkish",
    ],
    blocks: [
      {
        type: "bullets",
        title: "Personalization options",
        items: [
          {
            label: "Theme mode & accent palette",
            text: "Choose System, Light, or Dark mode under Settings › Appearance and track the Windows accent color or pick any custom OKLCH accent with automatic WCAG ≥ 4.5:1 contrast.",
          },
          {
            label: "Font scaling",
            text: "Switch between Small (12px), Normal (14px), Large (16px), and Extra Large (18px) across the entire interface.",
          },
          {
            label: "Live language switching",
            text: "Switch between Türkçe and English under Settings › Language; all windows update instantly without restarting.",
          },
        ],
      },
    ],
  },
  {
    id: "faq",
    title: "Frequently Asked Questions",
    summary:
      "Answers about non-expanding abbreviations, administrator privileges, password field safety, and Windows SmartScreen.",
    keywords: [
      "faq",
      "issue",
      "not working",
      "admin",
      "administrator",
      "password",
      "security",
      "smartscreen",
      "update",
    ],
    blocks: [
      {
        type: "faq",
        items: [
          {
            q: "My abbreviation is not expanding. Why?",
            a: "Verify the top status pill shows Active (not Paused). Check that the target app is not in Settings › Excluded apps and that the phrase has no restricting app rules. For elevated apps, enable 'Run as administrator (UAC-free Task)' in Settings › General.",
          },
          {
            q: "Why does Shift+F3 not change text in some fields?",
            a: "Shift+F3 only operates on selectable, editable text fields. Read-only web pages, PDF viewers, and secure password boxes are skipped intentionally.",
          },
          {
            q: "Does Meetin read what I type in password fields?",
            a: "No. Fields marked as password inputs via Windows UI Automation (IsPassword) are automatically skipped; keystrokes are never buffered or saved to clipboard history.",
          },
          {
            q: "What should I do if Windows SmartScreen warns during installation?",
            a: "Unsigned open-source installers may trigger a SmartScreen prompt on first launch. Click 'More info' → 'Run anyway' to proceed safely.",
          },
          {
            q: "How do I move my library to another computer?",
            a: "Export your library as a .mee rich archive from the top-right ⋯ menu (embedded images included) or restore a SQLite backup from Settings › Backup.",
          },
        ],
      },
    ],
  },
];

export function getHelpSections(lang: Language = getLanguage()): HelpSection[] {
  return lang === "en" ? HELP_SECTIONS_EN : HELP_SECTIONS;
}

function collectBlockText(block: HelpBlock): string {
  switch (block.type) {
    case "paragraph":
      return block.text;
    case "steps":
      return [block.title ?? "", ...block.items].join(" ");
    case "bullets":
      return [
        block.title ?? "",
        ...block.items.map((item) => `${item.label} ${item.text}`),
      ].join(" ");
    case "variables":
      return [
        block.title ?? "",
        ...block.items.map((item) => `${item.code} ${item.example} ${item.desc}`),
      ].join(" ");
    case "table":
      return [
        block.title ?? "",
        ...block.headers,
        ...block.rows.flat(),
      ].join(" ");
    case "tip":
      return block.text;
    case "faq":
      return block.items.map((item) => `${item.q} ${item.a}`).join(" ");
  }
}

export function searchHelpSections(
  query: string,
  lang: Language = getLanguage(),
): HelpSection[] {
  const sections = getHelpSections(lang);
  const locale = lang === "en" ? "en-US" : "tr-TR";
  const q = query.trim().toLocaleLowerCase(locale);
  if (!q) return sections;

  return sections.filter((section) => {
    const haystack = [
      section.title,
      section.summary,
      ...section.keywords,
      ...section.blocks.map(collectBlockText),
    ]
      .join(" ")
      .toLocaleLowerCase(locale);

    return haystack.includes(q);
  });
}
