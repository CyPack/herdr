---
doc: herdr-analysis
domain: upstream-sync
subject: fork ↔ upstream 125-commit senkronunun çakışma ölçümü
created: 2026-07-25
method: git fetch + rev-list + merge-tree (salt-okuma) + izole worktree probe
status: ölçümler executable (conf 0.95); strateji önerisi analiz (conf 0.8)
git_note: /docs/* gitignored → lokal. Makine kopyası ~/.cartography/herdr-analysis/
agentic_triggers:
  - "upstream merge · senkron · çakışma · conflict · cherry-pick · v0.7.4 · v0.7.5"
related:
  - docs/analysis/2026-07-25-license-impact-agpl-to-apache.md
  - docs/analysis/2026-07-24-architecture-seams.md
---

# Upstream Senkron Keşfi — 2026-07-25

> Not: Bu dosya koordinatörün ölçümlerinden yazıldı (merge-recon agent'ı kesildi).
> Tüm sayılar `git` komut çıktısıdır.

## Ölçüm

| Metrik | Değer |
|---|---|
| upstream/master | `38d2b078` (fetch 2026-07-25) |
| Bizde olmayan | **125 commit** |
| Upstream'de olmayan (biz) | **819 commit** |
| origin/master ↔ upstream | 777 commit |
| Ortak ata | `46174563` — 2026-07-11 |
| Yeni release | **v0.7.4, v0.7.5** |
| Drift hızı | 13 günde 125 ≈ ayda ~280 |

## Çakışma: `merge-tree --write-tree feat/native-fm upstream/master` → **exit=1**

**19 dosyada CONFLICT**, 50 dosya iki tarafta değişmiş:

```
justfile · src/app/actions.rs · agents.rs · api/agents.rs · api/plugins/mod.rs
input/mod.rs · input/modal.rs · input/mouse.rs · app/mod.rs · app/state.rs
kitty_graphics.rs · main.rs · persist/restore.rs · platform/mod.rs
server/headless.rs · terminal/state.rs · ui.rs · ui/panes.rs · ui/sidebar.rs
```

### Değişim büyüklüğü (ortak atadan)

| Dosya | UPSTREAM | BİZ |
|---|---|---|
| `src/ui.rs` | 60+/34- | **1960+/98-** |
| `src/app/state.rs` | 211+/4- | **1810+/13-** |
| `src/kitty_graphics.rs` | 462+/51- | 739+/8- |
| `src/app/input/mouse.rs` | 294+/41- | 680+/31- |
| `src/server/headless.rs` | **1137+/131-** | 405+/3- |
| `src/app/mod.rs` | **1180+/91-** | 396+/20- |

⇒ Her iki taraf da aynı dosyalarda yüzlerce–binlerce satır değiştirmiş. Bu **küçük bir merge değil**.

## Kritik upstream commit'leri

| Commit | Konu | Bizim için |
|---|---|---|
| `cd5ea1be` | **relicense apache-2.0** | LICENSE 872 satır. → lisans raporu |
| `36de78dd` | **preserve kitty graphics during host repaints** | 🎯 `render_stream.rs` + `headless/tests/pane_graphics.rs`. Tam bizim alan; işimize yarar |
| `1d238bc9` | replace pane mouse/clipboard internals | ⚠️ `pane/osc.rs` 471 satır sökülmüş. **Ama `osc.rs` çakışma listesinde YOK** → biz ona dokunmamışız (iyi haber) |
| `503613c8` | inherit host palette in pane queries | 15 dosya |
| `ef4c23f5` | release: v0.7.5 | — |

## Sıralama kararı: **önce merge, sonra kod**

Gerekçe (iki bağımsız kaynak):
1. Şerit 1 tam olarak `trail_snapshots.rs` + `preview_capability.rs` + `kitty_graphics.rs`'e dokunacak; upstream de `kitty_graphics.rs`'e 462 satır dokunmuş → sonra merge = aynı çakışmayı iki kez yaşamak.
2. `provider-source` raporu §G.7 bağımsız olarak aynı sonuca vardı.

## Strateji seçenekleri (ölçülmedi — kademeli merge testi kesinti yüzünden tamamlanmadı)

| # | Strateji | Risk | Not |
|---|---|---|---|
| A | Tek seferde tam merge (izole worktree'de) | Yüksek | 19 çakışma, en hassas dosyalar |
| B | Kademeli: `v0.7.4` → `v0.7.5` | Orta | **ÖLÇÜLMEDİ** — `merge-tree feat/native-fm v0.7.4` koşulmalı |
| C | Seçici cherry-pick (`36de78dd`) | Düşük | Drift devam eder; `kitty_graphics.rs`'te 739 satır değiştirmişiz → cherry-pick de çakışabilir |
| D | Merge yok, bağımsız fork | — | Upstream fix'leri (Windows, kitty) kaçar |

**Öneri:** izole worktree'de **B'yi önce ölç** (`merge-tree feat/native-fm v0.7.4` → kaç çakışma?), kademeli azaltıyorsa B, azaltmıyorsa A.

## Durum (kesinti sonrası, doğrulanmış)

```
/home/user/projects/herdr                b48bd903 [feat/native-fm]      ← DOKUNULMADI, temiz
/home/user/projects/herdr-upstream-recon b48bd903 [recon/upstream-sync-probe] ← temiz, MERGE_HEAD yok
```
Recon worktree **yerinde bırakıldı** — karar verilirse orada devam edilir.
Rollback: her şey `b48bd903`; worktree silinirse `git worktree remove` yeter.

## Açık / ölçülmemiş

- ⚠️ Kademeli merge (v0.7.4 ara adımı) çakışmayı azaltıyor mu — **ölçülmedi**
- ⚠️ Cherry-pick `36de78dd` çakışır mı — **ölçülmedi**
- ⚠️ Çakışmaların hunk-düzeyi zorluk sınıflandırması (mekanik/yapısal/semantik) — **yapılmadı**
- ⚠️ `just check` merge sonrası — **koşulmadı**

---
*v1.0.0 — 2026-07-25 · Ölçümler `git` çıktısı; strateji analiz.*
