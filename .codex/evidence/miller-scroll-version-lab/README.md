# Miller Scroll Version Lab

Bu klasör Miller Trail yatay kaydırma davranışının dört kalıcı Git
checkpoint'ini yan yana tutar. Snapshot'lar `git archive` ile doğrudan ilgili
commitlerden çıkarılmıştır; çalışma ağacının veya geçici runtime state'inin
kopyası değildir. Reboot bu kaynakları değiştirmez.

## Sürümler

| Sürüm | Closure commit | Product parent | Davranış |
|---|---|---|---|
| `v0-trail-baseline-3bd32bcf` | `3bd32bcff210d9f129365ad958459eaaaf561363` | `3c36f104c23d7eae81e1d87eded32ac57764e0c4` | Trail entegrasyonu kapalı; post-closure scroll düzeltmesi yok |
| `v1-horizontal-viewport-0f958efe` | `0f958efe062594e5ee3f67b478aa0112b8ef5be0` | `35c1393c3ff270a0ae79ac005ab552a23e927f68` | Manuel yatay origin frame'ler arasında korunuyor |
| `v2-fractional-one-third-84092e52` | `84092e52d111788f6d10ff5bcefa8d7d234aa02b` | `26da243712f5f79c3eda49cd07a88892f775246d` | Mutlak hücre ofseti, kısmi kolon clipping'i ve kolon genişliğinin 1/3'ü adım |
| `v3-plain-wheel-fallback-6a972703` | `6a972703113b473babd26a6ab18d14d1c937ac46` | `051f28294b259106a7e2f36e213c37229f0ec7d2` | Boş canlı kolon gövdesinde modifier'sız wheel mevcut 1/3 reducer'a düşüyor |

Her sürüm aynı sekiz scroll-otoritesi dosyasını içerir:

- `src/app/file_manager_miller.rs`
- `src/app/input/file_manager.rs`
- `src/fm/miller.rs`
- `src/ui.rs`
- `src/ui/file_manager.rs`
- `src/ui/file_manager/miller.rs`
- `src/ui/file_manager/trail_view.rs`
- `src/ui/visual_fixture.rs`

## Diff inceleme komutları

Komutlar repo kökünden çalıştırılır ve hiçbir dosyayı değiştirmez:

```bash
git diff --no-index \
  .codex/evidence/miller-scroll-version-lab/v0-trail-baseline-3bd32bcf/src \
  .codex/evidence/miller-scroll-version-lab/v1-horizontal-viewport-0f958efe/src
```

```bash
git diff --no-index \
  .codex/evidence/miller-scroll-version-lab/v1-horizontal-viewport-0f958efe/src \
  .codex/evidence/miller-scroll-version-lab/v2-fractional-one-third-84092e52/src
```

```bash
git diff --no-index \
  .codex/evidence/miller-scroll-version-lab/v2-fractional-one-third-84092e52/src \
  .codex/evidence/miller-scroll-version-lab/v3-plain-wheel-fallback-6a972703/src
```

`git diff --no-index` fark bulduğunda exit `1` döndürür; bu hata değil, diff
bulunduğu anlamına gelir.

## Sonraki ranking kontratı

Ranking kaynak satır sayısına veya “en yeni commit” varsayımına göre
yapılmayacak. Her sürüm aynı ağırlıklı matriste değerlendirilecek:

| Boyut | Ağırlık |
|---|---:|
| Canlı mouse/terminal doğruluğu | 25 |
| Trail state, render ve input için tek otorite | 20 |
| Kısmi kolon clipping ve hit-test doğruluğu | 15 |
| Resize, rebranch ve auto-follow regresyonları | 15 |
| Stale generation/revision fail-closed davranışı | 10 |
| Playwright Chromium görsel kanıtı | 10 |
| Karmaşıklık ve geri alma maliyeti | 5 |

Production-grade kazanan ancak aynı isolated runtime matrisi, Rust regresyon
aileleri ve Playwright Chromium karşılaştırması dört sürüm için de
çalıştırıldıktan sonra seçilebilir.
