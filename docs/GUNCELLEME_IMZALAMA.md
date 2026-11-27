# Güncelleme imzalama

Meetin güncellemeleri `tauri-plugin-updater` ile denetlenir ve kurulur (şartname §9).

## Nasıl çalışır

1. Uygulama açılışta ve 24 saatte bir `https://github.com/ismetuluer/meetin/releases/latest/download/latest.json` adresini okur.
2. Yeni sürüm varsa kurulum dosyasını indirir ve **minisign imzasını** uygulamanın içindeki açık anahtarla doğrular (`src-tauri/tauri.conf.json` › `plugins.updater.pubkey`).
3. İmza tutmazsa ya da yoksa kurulum yapılmaz; hata kayıt günlüğüne `update.install_failed` olarak yazılır.
4. Portable sürüm kendini kurmaz; yeni ZIP'in adresi gösterilir.

## Anahtar

| Dosya (yalnız Saul'un bilgisayarında) | Nerede kullanılır |
| --- | --- |
| `%USERPROFILE%\.tauri\meetin-updater.key` | GitHub secret `TAURI_SIGNING_PRIVATE_KEY` |
| `%USERPROFILE%\.tauri\meetin-updater.password.txt` | GitHub secret `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` |
| `%USERPROFILE%\.tauri\meetin-updater.key.pub` | `tauri.conf.json` içinde (herkese açık) |

- Özel anahtar ve parola depoya girmez, yedekleri ayrı yerlerde tutulur.
- Anahtar kaybolursa kurulu sürümler yeni güncellemeleri doğrulayamaz; yeni anahtarla çıkan sürüm bir kez elle kurulmalıdır.

## Yayın (CI)

`vX.Y.Z` etiketi `.github/workflows/release.yml`'yi çalıştırır:

1. Kurulum dosyası ve portable ZIP derlenir (varsa Authenticode imzası atılır).
2. `npx tauri signer sign` kurulum dosyasını imzalar; secret yoksa yayın durur.
3. `latest.json` yazılır: eklenti alanları (`version`, `notes`, `pub_date`, `platforms`) ve 1.4.1 ve öncesinin okuduğu köprü alanları (`download_url`, `sha256`, `portable_url`, `portable_sha256`).
4. Notlar `CHANGELOG.md`'deki sürüm bölümünden alınır; bölüm yoksa yayın durur.
