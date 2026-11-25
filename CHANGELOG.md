# Değişiklik Günlüğü

Biçim: [Keep a Changelog](https://keepachangelog.com/tr-TR/1.1.0/) · Sürümleme: SemVer (bkz. `AGENTS.md`).

## [0.0.1] - 2026-11-25

### Eklendi

- Proje iskeleti: Tauri 2 + React 19 + TypeScript (Vite 8), Rust çekirdeği `meetin`.
- Şartname §10 klasör yapısı: `src-tauri/src/{hook,matcher,inject,selection,vars,db,io,windows}`, `src/{mascot,island,bubble,studio,design,i18n}`.
- Bağımlılıksız arayüz metni yardımcısı `t()` ve `src/i18n/tr.json`.
- Test ve kalite araçları: `cargo test`, `cargo clippy`, `rustfmt`, Vitest, ESLint, Prettier.
- Yol haritası (`docs/YOL_HARITASI.md`) ve çalışma kuralları (`AGENTS.md`).
