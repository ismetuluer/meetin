# Meetin — İmzalı Güncelleme ve Kod İmzalama Rehberi

Bu rehber, Meetin'in tek tıkla doğrulamalı öz-güncelleme mekanizmasının nasıl çalıştığını, kendi imzalama anahtar çiftinizi nasıl üreteceğinizi ve GitHub Actions yayın hattına nasıl bağlayacağınızı açıklar.

---

## 1. Mimari Özet

Meetin, hiçbir ek harici kripto kütüphanesi gerektirmeden (çevrimdışı derleme uyumlu) iki katmanlı bütünlük ve kaynak doğrulaması uygular:

1. **SHA-256 Bütünlük Doğrulaması:** İndirilen NSIS kurulum paketi veya Portable arşivi akış halinde SHA-256 ile özetlenir ve `latest.json` içindeki beklenen `sha256` / `portable_sha256` değeriyle sabit zamanlı olarak bayt bayt karşılaştırılır. Tek bir bayt bile uyuşmazsa dosya silinir ve kurulum iptal edilir.
2. **ECDSA P-256 (NIST P-256) Kriptografik İmza Doğrulaması:** Manifestteki `signature` alanı, aşağıdaki kanonik yükün (UTF-8, `LF` satır sonu, küçük harfli hex `sha256`) Windows CNG (`BCryptVerifySignature`) üzerinden doğrulanmasıyla kontrol edilir:

```text
<version>
<sha256>
<download_url>
```

- **İmza geçerliyse (`verified`):** Ayarlar › Güncelleme ekranında **Doğrulandı** rozeti görünür ve tek tıkla **İndir ve Güncelle** etkinleşir.
- **İmza yoksa (`unsigned`):** CI sırrı henüz tanımlanmamış sürümlerde **İmzasız** rozeti gösterilir; kurulum sırasında HTTPS ve SHA-256 bütünlük doğrulaması yine zorunlu tutulur.
- **İmza geçersizse (`invalid`):** Sürüm, özet veya adres değiştirilmişse ya da sahte imza varsa **Geçersiz imza** uyarısı gösterilir, düğme devre dışı bırakılır ve kurulum kesinlikle engellenir.

### Kurulum Modları

- **Kurulumlu (NSIS):** Doğrulanmış kurulum paketi geçici klasörde hazırlanır, uygulama güvenlik yedeğini alıp kapanır, NSIS sessiz argümanıyla (`/S`) günceller ve Meetin'i otomatik yeniden başlatır.
- **Portable:** Doğrulanmış `Meetin.exe` (veya `.zip` içindeki `Meetin.exe`) mevcut çalıştırılabilir dosyanın yerine kopyalanırken eski sürüm `Meetin.old.exe` olarak kenara alınır; `data\` klasörüne kesinlikle dokunulmaz.

---

## 2. İmzalama Anahtar Çifti Üretme (Yerel Bilgisayarınızda)

Proje kök klasöründen şu betiği çalıştırın (varsayılan olarak `$HOME\.meetin-signing-keys` klasörüne — Git deposunun **dışına** — yazar):

```powershell
./scripts/generate-signing-keys.ps1
```

Üretilen dosyalar:
- `meetin-signing.private.pem` — **GİZLİ ÖZEL ANAHTAR.** Asla Git deposuna eklemeyin, paylaşmayın veya ekran görüntüsünde göstermeyin.
- `meetin-signing.public.pem` — Standart X.509 SPKI PEM açık anahtarı.
- `meetin-signing.public.b64` — Derleme ortam değişkeni için tek satırlık Base64 açık anahtar.

---

## 3. GitHub Actions Sırlarını (Secrets) Tanımlama

Depo ayarlarında **Settings › Secrets and variables › Actions** bölümüne gidin:

| Ad | Tür | Zorunlu mu? | Açıklama |
| --- | --- | --- | --- |
| `MEETIN_SIGNING_PRIVATE_KEY` | Secret | İsteğe bağlı | `meetin-signing.private.pem` dosyasının tam içeriği. Tanımlıysa `latest.json` otomatik imzalanır; tanımlı değilse CI imzasız devam eder. |
| `MEETIN_UPDATE_PUBLIC_KEY` | Variable veya Secret | İsteğe bağlı | `meetin-signing.public.b64` içindeki tek satırlık Base64 açık anahtar. Derleme sırasında uygulamaya gömülür. |
| `MEETIN_SIGNING_KEY_ID` | Variable | İsteğe bağlı | Anahtar kimliği etiketi (varsayılan: `meetin-release-1`). |
| `WINDOWS_CERT_PFX_BASE64` | Secret | İsteğe bağlı | Windows Authenticode kod imzalama sertifikasının (`.pfx`) Base64 metni. |
| `WINDOWS_CERT_PASSWORD` | Secret | İsteğe bağlı | `.pfx` sertifika parolası. Her iki sertifika sırrı da tanımlıysa `signtool.exe` ile NSIS kurulumu ve `meetin.exe` imzalanır; tanımlı değilse adım nazikçe atlanır. |

---

## 4. Manuel Manifest İmzalama

Yerel bir `latest.json` dosyasını elle imzalamak için:

```powershell
./scripts/sign-manifest.ps1 `
  -ManifestPath "./dist-packages/latest.json" `
  -PrivateKeyPath "$HOME\.meetin-signing-keys\meetin-signing.private.pem" `
  -KeyId "meetin-release-1"
```

Veya özel anahtarı ortam değişkeninden okumak için:

```powershell
$env:MEETIN_SIGNING_PRIVATE_KEY = Get-Content -Raw "$HOME\.meetin-signing-keys\meetin-signing.private.pem"
./scripts/sign-manifest.ps1 -ManifestPath "./dist-packages/latest.json"
```

Örnek tam `latest.json` biçimi için [latest.json.example](./latest.json.example) dosyasına bakın.
