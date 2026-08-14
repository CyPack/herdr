---
doc: herdr-analysis
domain: vision-mission
subject: ürün vizyonu, misyon durumu, faz haritası, fork↔upstream ayrışması
created: 2026-07-24
method: README/website/blog/SPONSORS/CONTRIBUTING + .codex continuity + .local/prd + git ölçümleri (rev-list) + codebase-memory-mcp doğrulaması
status: canonical — her iddia (claim, evidence, confidence); test gate iddiaları korelasyonlu belge (conf 0.6) olarak İŞARETLİ
git_note: >
  /docs/* herdr .gitignore'da IGNORED (yalnız /docs/next/ istisna) → lokal yaşar, upstream'e sızmaz.
  Makine kopyası: ~/.cartography/herdr-vision-mission-*
agentic_triggers:
  - "vizyon · misyon · ürün hedefi · roadmap · faz · yol haritası"
  - "fork · upstream · ogulcancelik · CyPack · divergence · release kanalı"
  - "SF fazları · FM fazları · FIP · FMN · FMH · FFO · DCLICK · change pipeline"
  - "neden yapıyoruz · scope · öncelik · kanonik görev önceliği"
  - "mimari anayasa · runtime client boundary · render purity · platform izolasyonu"
related:
  - docs/analysis/2026-07-24-chat-forensics-codex-cursor-handover.md
  - docs/analysis/2026-07-24-decision-matrix-and-roadmaps.md
  - .codex/CURRENT.md
  - .codex/TASKS.md
  - .local/prd/custom-layout-target-mockup.md
---

# herdr — Vizyon & Misyon Durum Analizi

**Analiz tarihi:** 2026-07-24 · **Repo:** `/home/ayaz/projects/herdr` · **HEAD:** `b48bd903` (`feat/native-fm`)
**Kapsam:** salt-okuma inceleme (kod değişikliği yapılmadı)
**Yöntem:** discovery-first + evidence-propagation. Her iddia (claim · evidence · confidence) üçlüsüyle kaydedildi. Kod keşfinde önce codebase-memory graph (24.357 node / 129.892 edge, `status: ready`, taze sembol `TrailSnapshots::focus_entry` ile freshness doğrulandı), sonra grep.

---

## İÇİNDEKİLER

1. [Dört Temel Soruya Doğrudan Cevap](#0-dört-temel-soruya-doğrudan-cevap)
2. [A. Vizyon — herdr ne olmak istiyor](#a-vizyon--herdr-ne-olmak-istiyor)
3. [B. Misyon — bugün fiilen ne yapıyor](#b-misyon--bugün-fiilen-ne-yapıyor)
4. [C. Mimari Anayasa ve kodda tutulma doğrulaması](#c-mimari-anayasa-ve-kodda-tutulma-doğrulaması)
5. [D. Faz haritası ve tamamlanma oranları](#d-faz-haritası-ve-tamamlanma-oranları)
6. [E. Vizyon ↔ Gerçeklik boşluk grid'i](#e-vizyon--gerçeklik-boşluk-gridi)
7. [F. Stratejik riskler ve açık kararlar](#f-stratejik-riskler-ve-açık-kararlar)
8. [G. Sıradaki mantıklı adımlar](#g-sıradaki-mantıklı-adımlar)
9. [H. KARAR GEÇMİŞİ ARŞİVİ](#h-karar-geçmişi-arşivi)
10. [I. Vizyon soruları yeniden açılırsa okunacak kaynaklar](#i-vizyon-soruları-yeniden-açılırsa-okunacak-kaynaklar)
11. [J. Bu turda İNCELENMEYEN vizyon eksenleri](#j-bu-turda-i̇ncelenmeyen-vizyon-eksenleri)
12. [K. Kanıt sözleşmesi ve ölçüm notları](#k-kanıt-sözleşmesi-ve-ölçüm-notları)

---

## 0. DÖRT TEMEL SORUYA DOĞRUDAN CEVAP

### 0.1 — Vizyon tek paragrafta + fork ayrışması

**herdr'ın vizyonu (kendi beyanıyla):**

herdr, "**agent multiplexer that lives in your terminal**" (`README.md:25`) olarak konumlanan, Cargo metadata'sında "**terminal workspace manager for AI coding agents**" (`Cargo.toml:6`) diye tanımlanan, nihai hedefi ise "**becoming the runtime for coding agents**" (`SPONSORS.md:3-4`) olan bir üründür. Vizyonun felsefi çekirdeği kendi blog manifestosunda yatar: coding agent CLI'ları artık chat arayüzü değil **runtime**'dır ("They pause, resume, compact, fail, recover... **This is runtime behavior.**" — `website/src/content/blog/coding-agents-are-becoming-runtimes.md:11-15`); model ve API tescilli olabilir ama **"terminal, sahipliğin paylaşıldığı yerdir"** (satır 71) ve **"herdr agent'ın ALTINDA durur"** (satır 116) — pane'lere sahiptir, process'leri yaşatır, agent'ın lifecycle'ını dışarıdan gözlemlenebilir kılar. Sektöre çağrısı: "**Standardize the boring parts**" (satır 157) — tek proje talimat formatı, küçük bir `working/blocked/idle` lifecycle sözleşmesi, gözlemlenebilir izin durumu.

**Fork nerede ayrışıyor:**

CyPack fork'unun beyan edilmiş **birinci önceliği tamamen farklı bir yerde**:

> **"Priority #1 of the whole effort: a file manager better than yazi and superfile"**
> — `.local/prd/custom-layout-target-mockup.md:5-6` (kullanıcı direktifi, 2026-07-18)

> **"yazi'ye İHTİYACIMIZ YOK. Dosya yöneticisini herdr'a NATIVE (Rust/ratatui, Lua'sız) inşa etmek hem daha temiz hem hedefe daha uygun."**
> — `.local/prd/native-file-manager-DECISION.md:13` (6 paralel cartographer kanıtıyla verilmiş karar, 2026-07-13)

İkinci fork ekseni: **custom layout altyapısı** — TopBar / LeftPanel×2 / CenterStage-tabs / RightRail / RightPanel / BottomBar bölgelerinden oluşan, kullanıcının Excalidraw mockup'ıyla tanımladığı yapılandırılabilir kabuk (`.local/prd/custom-layout-target-mockup.md:11-19`).

**Native file manager upstream'in HİÇBİR yayınlanmış vizyon belgesinde geçmiyor** — `README.md`, `website/src/pages/compare.astro`, `website/src/content/docs/concepts.mdx`, `docs/next/CHANGELOG.md` taramalarında ürün özelliği olarak yer almıyor (negatif kanıt, confidence 0.8).

| Boyut | Upstream (`ogulcancelik/herdr`) | CyPack fork (`origin`) |
|---|---|---|
| Birincil hedef | Agent runtime olmak | **yazi/superfile'ı aşan native file manager** |
| İkincil eksen | Agent detection + socket API + marketplace | **Custom layout altyapısı** (shell bölgeleri) |
| Sürüm disiplini | stable/preview kanalları, GitHub Release | Yayın YOK — sadece fork branch'lerine FF push |
| Doküman yüzeyi | herdr.dev/docs + `docs/next/` | `.codex/` + `.local/` + `docs/superpowers/` (çoğu gitignored) |
| Karar mercii | ogulcancelik (solo maintainer) | Kullanıcı (Ayaz/CyPack) direktifi |

**Sonuç:** İki vizyon **çatışmıyor ama örtüşmüyor**. Fork, upstream'in mimarisini (Shell/Stage/Compositor) miras alıp üzerine upstream'de talep edilmemiş bir ürün ekseni inşa ediyor.

### 0.2 — "master 742 commit geride" — DÜZELTME: FORK İLERİDE

Ölçüm (deterministik, `git rev-list --count`):

| Metrik | Değer |
|---|---|
| `origin/master`'da olup `upstream/master`'da OLMAYAN | **777 commit** |
| `upstream/master`'da olup `origin/master`'da OLMAYAN | **4 commit** |
| `feat/native-fm` → `origin/master` ilerisi | **42 commit** |
| **Toplam fork divergence** | **819 commit** |
| Ortak ata (merge-base) | `46174563` — 2026-07-11 02:46 |
| 777 commit'in tarih aralığı | 2026-07-11 → 2026-07-19 (**8 gün**) |
| 777 commit'in yazarı | **CyPack — %100 (777/777)** |
| Tip dağılımı | test 321 · feat 194 · docs 174 · fix 64 · refactor 17 · style 2 · chore 2 · perf 1 · build 1 |

**Cevap: FORK İLERLEDİ.** herdr master'ı geride değil — CyPack fork'u upstream'den 819 commit önde ve upstream'in yalnızca 4 commit'ini almamış (`4ca6cac4 fix: match homepage mock pane chrome`, `a6905364 fix: clarify config fallback diagnostics`, `749e85e0 fix: detach windows server from host terminal`, `e418e4a4 chore: remove unmerged approved contributors`). "742 geride" okuması terstir — bu tuzağı §I.4'te kayda geçirdim.

**Misyonun üç katmanı:**

- **YAYINLANMIŞ (v0.7.3 stable, upstream mirası):** workspace/tab/pane multiplexing · kalıcı PTY + detach/reattach · 19 agent detection manifest (hot-reload) · semantik agent durumu · socket API + CLI (protocol v16) · plugin sistemi + Cloudflare Worker marketplace · git worktree · SSH/remote · Windows preview beta · kitty graphics · stable+preview kanalları · en/ja/zh-cn docs
- **FORK BRANCH'İNDE BEKLİYOR (819 commit, hiç yayınlanmadı):** native file manager (`src/fm/` 14 dosya + `src/app/input/file_manager.rs` 10.833 satır) · Shell bölge sistemi (`src/ui/shell/` 5 dosya, `RegionRects::get` fan_in 374) · typed Stage/SurfaceHost (`StageSurfaceView`, fan_in 47) · AppDock · shell input router · Miller sütunlar + yatay viewport + sütun resize · metin/görsel preview · dosya işlemleri · dosya→agent handoff · fs watcher · Locations rail · Playwright görsel altyapı (VIS-01..27) · snapshot v4 persistence
- **SADECE PLANDA:** Change Pipeline T3-T10 (duraklatıldı) · Custom Layout B1-B4 (hiç başlamadı) · PDF/XLSX render (plugin yolu tasarlandı, uygulanmadı) · dosya edit (sıfır kod) · S5/S7/M3 (kalıcı NO-GO)

### 0.3 — Belge render/edit + custom layout vizyonun neresinde

**A) Belge render/edit**

Mevcut durum (`src/fm/preview_capability.rs`):

| Kategori | Formatlar | Bugünkü davranış | Kanıt |
|---|---|---|---|
| Native görsel | png, jpg, gif, webp vb. | **Gerçek render** (kitty graphics motoru) | `src/fm/image_preview.rs` 734 satır; B0 spike → B2 kapalı |
| Native metin | txt, kod, md/markdown/mdown | **Bounded metin render** | `src/fm/text_preview.rs` 307 satır; `preview_capability.rs:123` |
| **Belge** | **pdf, doc, docx, odt, rtf, xls, xlsx, ods, ppt, pptx, odp** | **YALNIZCA metadata** — `PreviewReason::DocumentMetadata` → görünen metin: `"optional document viewer"` | `preview_capability.rs:129` + satır 33 |
| Arşiv / medya / binary | — | Yalnızca metadata | `preview_capability.rs:34-36` |

**Düzenleme (edit):** `grep -rniE "fn .*(edit|write_file|save_file|editor)"` → `src/fm/` ve `src/app/file_*.rs` içinde **sıfır sonuç**. Mevcut mutasyonlar yalnızca: `execute_file_operation` (copy/move), `execute_rename_operation`, `execute_bulk_rename_operation`, `execute_delete_operation`. İçerik düzenleme kod yolu **yok** (confidence 0.9 — negatif grep + fonksiyon envanteri çapraz-kontrolü).

**KRİTİK UYARI — mevcut mimari karar bu talebe ters bakıyor:**

`docs/superpowers/plans/2026-07-18-herdr-files-visibility-preview-plugin-integration.md` "Task 5: Add an optional plugin preview adapter":

> "...opens **a plugin pane** and **never injects renderer output into native Ratatui cells**."

FMR-5 seçilmiş sınır (`.codex/TASKS.md`):

> "Select hybrid boundary: **native core owns directory/path/Trail/mouse truth and lightweight bounded preview; optional plugins own heavyweight expert panes.**"

Yani proje, ağır belge görüntüleyicileri (PDF/XLSX) için **native render'ı bilinçli olarak REDDETMİŞ**, bunun yerine plugin-pane yolunu seçmiş. Bu karar `.codex/evidence/files-preview-capability-test-points.md` ile kanıtlanmış ve FMR-3 kapanışında dondurulmuş.

| Alt-özellik | Doğal uzantı mı? | Gerekçe |
|---|---|---|
| PNG render | **Zaten var** — genişletme değil | B0/B1/B2 kapalı; `image_preview.rs` çalışıyor |
| PDF/XLSX **plugin-pane** render | **Vizyonun doğal uzantısı** | FMR-5 P5 zaten planlı; `src/app/file_preview_plugin.rs` dosya adı bile planda belirlenmiş. Plugin ekosistemi upstream vizyonunun parçası (`README.md:31`) |
| PDF/XLSX **native** render | **Scope genişlemesi + mevcut karara aykırı** | FMR-5 hybrid sınırı native ağır render'ı dışlıyor. Yapılacaksa bu karar **açıkça geri alınmalı** (yeni PRD + kullanıcı onayı) |
| Dosya **edit** | **Belirgin scope genişlemesi** | Ne upstream ne fork belgelerinde geçiyor. herdr multiplexer, editör değil. En yakın mevcut desen: `prefix+e` scrollback editör pane'i (`$VISUAL`/`$EDITOR` başlatır) → herdr **editörü barındırır, editör olmaz**. Bu deseni izlemek vizyonla uyumlu; kendi editörünü yazmak değil |

**Upstream kapsamıyor** — %100 CyPack fork'una özgü. **Faz yeri:** FMR-5 P5 (plugin adapter); önkoşulları P0-P4 **kapalı**.

**B) Custom layout**

Taşıyıcı katmanların **TAMAMI kapalı**:

| Bileşen | Durum | Kod |
|---|---|---|
| Bounded named-region solver | ✅ SF2 | `src/ui/shell/layout.rs`, `model.rs` |
| Typed template + nesting | ✅ SF2 | `src/ui/shell/template.rs` |
| Cached generation-safe `ShellView` | ✅ SF2.4 | `src/ui/shell/view.rs`; `RegionRects::get` fan_in 374 |
| Paylaşılan `ResizeTransaction` (region-generic) | ✅ SF3 | dock için 3..=9 pinned |
| Collapse/restore + iki-eksen scroll | ✅ SF3.2 | |
| Snapshot v4 persistence + v3 migration | ✅ SF3.3 | `src/persist/` |
| Overlay/capture input router | ✅ SF4.2 | `src/app/input/shell.rs::route_shell_input` |
| Typed Stage surfaces | ✅ SF4.1/SF6 | `src/ui/surface_host.rs` |
| AppDock (region-generic dock track) | ✅ SF5 | `src/ui/app_dock.rs` |

**Eksik:** 7 mockup bölgesinden **3'ünün gerçek tüketicisi yok**:

| Bölge | Mevcut seam | Durum |
|---|---|---|
| TopBar | yeni template track gerekli | ❌ tüketici yok |
| LeftPanel (üst+alt) | mevcut LeftPanel + dikey split | ⚠️ split solver destekliyor, içerik component yok |
| CenterStage + tab strip | WorkspaceStage + typed surfaces ✅ | ⚠️ stage-local tab strip yok |
| RightRail (dikey ikon şeridi) | AppDock pattern sağa döndürülmüş | ❌ yok |
| RightPanel | ShellLayout RightPanel | ❌ fixture var, **gerçek tüketici yok** (eski S6 tetiği) |
| BottomBar | ShellLayout BottomBar track | ❌ fixture var, tüketici yok |

**Doğal uzantı mı? EVET, fork vizyonunun doğrudan parçası** — scope genişlemesi değil. PRD kullanıcı direktifini kayda geçirmiş: *"build the custom-layout infrastructure so that composing this layout is EASY, FAST, and production-grade"* — file-manager hedefinin **altyapısı** olarak konumlandırılmış.

**Önkoşullar kapandı mı? EVET — %100:**

```
SF0 ✅ → SF1 ✅ → SF2 ✅ → SF3 ✅ → SF4 ✅ → SF5 ✅ → SF6 ✅   (TÜM ÖNKOŞULLAR KAPALI)
FM1 ✅ → FM2 ✅ → FM3 ✅ → FM4 ✅ → FM5 ✅                      (TÜM ÖNKOŞULLAR KAPALI)
                                    ▼
                    B1 ❌ → B2 ❌ → B3 ❌ → B4 ❌
                    (Custom Layout — HİÇ BAŞLAMADI)
```

Kanıt: `docs/superpowers/specs/2026-07-18-herdr-fip-closure-and-custom-layout-prd.md:37-42` B-zincirini tanımlıyor; `grep -i "custom layout" .codex/CURRENT.md` → **0 sonuç**; B2 design spec dosyası `docs/superpowers/specs/` listesinde **yok**. Confidence 0.9.

**KORUMA NOTU:** Bu iş P4.0'da reddedilen "arbitrary component registry" tuzağına düşmemeli. `docs/superpowers/specs/2026-07-17-herdr-custom-layout-architecture-guide.md:132` anti-pattern tablosunda açıkça: *"Arbitrary component registry for the layout → Over-engineering without a second consumer → P4.0 S5 NO-GO."* Custom layout **somut bölge tüketicileri** eklemeli, genel bir registry değil.

**Önerilen sıra:** (0) DCLICK-6 + FFO-9 fiziksel E2E kapat → (1) **Custom Layout B1→B2** (belge render'ın nereye oturacağını da bu belirler) → (2) FMR-5 P5 plugin adapter → (3) native render kararı (gerekirse, açık PRD ile) → (4) edit (en büyük genişleme, önce ürün kararı).

### 0.4 — Faz tamamlanma oranları

| Kayıt | Kapalı | Açık | Toplam | Oran |
|---|---|---|---|---|
| `.codex/TASKS.md` (ürün) | **468** | 24 | 492 | **%95,1** |
| `.codex/CHANGE-PIPELINE-TASKS.md` (araç lane) | **25** | 89 | 114 | **%21,9** |
| **Birleşik** | 493 | 113 | 606 | **%81,4** |

Ölçüm: `grep -oE '^\s*-?\s*\[[ x]\]' | sort | uniq -c` · confidence 0.95 (deterministik sayım).

---

## A. VİZYON — herdr ne olmak istiyor

### A.1 Kimlik katmanları

| # | İddia | Kanıt | Güven |
|---|---|---|---|
| V1 | Tek cümlelik konumlandırma: **"agent multiplexer that lives in your terminal"** | `README.md:25` · `docs/next/README.md:17` (aynı cümle) | verified 0.95 |
| V2 | Cargo metadata resmî tanımı: **"terminal workspace manager for AI coding agents"** | `Cargo.toml:6` | verified 0.95 |
| V3 | Nihai hedef (north star): **"the runtime for coding agents"** | `SPONSORS.md:3-4` "reaching herdr's goal of becoming the runtime for coding agents" · `README.md:58` "the path to a real agent runtime" | verified 0.9 |
| V4 | Ürün skill tanımı: **"terminal multiplexer and runtime for coding agents"** | `SKILL.md:8` | verified 0.9 |
| V5 | Marka/hero: "One terminal. The whole herd." — sürü metaforu (çok-agent yönetimi) | `website/index.html` hero | verified 0.9 |

**Sentez:** herdr, "terminal multiplexer" olarak başlayıp **"coding agent runtime"**e evrilmeyi hedefleyen bir üründür. Bugünkü kimliği multiplexer, hedeflenen kimliği runtime. Bu ikilik ürünün her belgesinde tutarlı biçimde tekrarlanıyor.

### A.2 Vizyon manifestosu — en derin niyet beyanı

`website/src/content/blog/coding-agents-are-becoming-runtimes.md` (yayın: 2026-06-10) herdr'ın en açık felsefi konum belgesi:

- **Teşhis (satır 11-15):** Coding agent CLI'ları artık chat arayüzü değil, **runtime davranışı** sergiliyor — "They read project instructions. They run commands. They ask for permission. They spawn subagents. They pause, resume, compact, fail, recover, and keep working while the developer does something else. That shape is familiar. **This is runtime behavior.**"
- **Sınır çizimi (satır 17-19):** "Model tescilli olabilir, hosted API tescilli olabilir, abonelik istenildiği gibi fiyatlanabilir. Ama geliştiricinin terminalinde çalışan CLI, **geliştirici ortamının parçasıdır**. Shell'lerin, language server'ların, package manager'ların, test runner'ların ve multiplexer'ların yanında durur."
- **Standart kayması uyarısı (satır 23-51):** JavaScript `package.json` + lockfile katmanları, Python `requirements.txt`/`setup.py`/`Pipfile`/`poetry.lock`/`pyproject.toml` tarihini örnek göstererek agent CLI'larının aynı parçalanmayı tekrarlamaması gerektiğini savunuyor. `AGENTS.md` standardı (60.000+ proje) ile Claude Code'un `CLAUDE.md` ısrarı arasındaki symlink workaround'ı "standart kaymasının başlangıcı" olarak işaretleniyor (satır 45-47).
- **Sahiplik tezi (satır 65-71):** *"What does a developer actually own: the subscription, the API spend, the model access, or the runtime where the agent works?"* → **"The terminal is where ownership gets shared."**
- **herdr'ın konumlandığı katman (satır 116-118):** **"Herdr sits below the agent."** — pane'lere sahiptir, process'leri yaşatır, geliştiricinin agent'lar arasında geçiş yapmasını/incelemesini/oturum kurtarmasını sağlar.
- **Kendi zayıflık itirafı (satır 122-137):** herdr'ın ilk agent tespit yolu **ekran okumaydı**; insanın gördüğüyle eşleştiği için işe yaradı ama agent UI'ları değişince kırıldı. Fix herdr binary'sinin içinde olsaydı, kullanıcı yeni bir prompt'u tanımak için tam güncelleme yapmak zorunda kalırdı → **hot-reload edilebilir detection manifest**'lere geçildi. Ayrıca "terminal aktivitesi = çalışıyor" sinyali de reddedildi: "A spinner can redraw while the meaningful state is blocked."
- **Şeffaflık sözü (satır 139-151):** herdr **per-agent support matrix** yayınlayacak — hangi agent doğrulanmış lifecycle sinyali veriyor, hangisi gözlemsel tespit gerektiriyor. *"This distinction should be public because it helps everyone."*
- **Sektöre çağrı (satır 155-175):** **"Standardize the boring parts."** Tek proje talimat formatı · küçük lifecycle sözleşmesi · gözlemlenebilir izin durumu · gözlemlenebilir kesinti durumu · state'i gerçek oturuma bağla · dış araçların neye güvenebileceğini dokümante et. Rekabet modelde, UX'te, planlamada, bağlam yönetiminde, hızda, fiyatta, güvenlikte ve zevkte olsun — "bir repo'nun beş talimat dosyasına ihtiyacı olup olmadığında değil."

Bu manifesto herdr'ın vizyonunun sadece bir ürün değil, bir **ekosistem standardı savunuculuğu** olduğunu gösteriyor.

### A.3 Hedef kitle

| Kitle | Kanıt | Güven |
|---|---|---|
| Terminalde çok sayıda coding agent çalıştıran bireysel geliştiriciler | `README.md:27-32` özellik listesi; `website/index.html` "Popular with engineers from / **Individual engineers, not company endorsements**" | verified 0.85 |
| tmux/zellij'den gelen multiplexer kullanıcıları | `website/src/content/docs/index.mdx:26-30` "Coming from tmux or zellij? You already know the model. The prefix is `ctrl+b`, panes persist, detach and reattach work the way you expect." | verified 0.9 |
| Multiplexer deneyimi OLMAYAN kullanıcılar (mouse-first giriş) | `index.mdx:22-25` "You don't need to learn shortcuts to start. Herdr is mouse-first: click panes, drag borders, split and switch from right-click menus." | verified 0.9 |
| **AI agent'ların kendisi** (birinci sınıf kullanıcı) | `README.md:29` "agents can use herdr too — a pure socket api: agents spawn panes, read output, wait on each other"; `SKILL.md` tümü; `website/agent-guide.md`; `docs/agent-skill.mdx` | verified 0.95 |
| Uzak/SSH ve mobil-genişlik terminal kullanıcıları | `docs/persistence-remote.mdx`; `src/ui/mobile.rs`; issue `#316` mobile-width-threshold | verified 0.85 |

**Agent'ların birinci sınıf kullanıcı olması, herdr'ı benzerlerinden ayıran en özgün kitle kararıdır.** `index.mdx:35-40` bunu onboarding'e bile taşımış: *"Already running an AI coding agent? Let it do the onboarding. Paste this prompt: Help me understand and set up Herdr. Read https://herdr.dev/agent-guide.md first..."*

### A.4 Rakiplerden farkı (`website/src/pages/compare.astro`, 478 satır)

Sayfanın tezi: **"Pick the center of gravity"** — herdr, dört kategorinin **kesişiminde** duruyor, hiçbirinin tam içinde değil. Alt başlık: *"The intersection other tools miss. Persistent terminal runtime, built for agents."*

| Rakip kategorisi | Örnekler | herdr'ın ayrım iddiası (birebir alıntı) |
|---|---|---|
| Klasik multiplexer | tmux, Zellij | "Classic multiplexers stop at terminals. They own panes and persistence. **They do not know which pane is blocked, working, done, or ready for an agent wait.**" |
| Terminal-yerine-geçen app | Warp, cmux | "Terminal apps replace the terminal. They can add polished desktop UX, but the workflow moves into their app. **Herdr stays inside the terminal you already use.**" |
| Process dashboard | Solo | "Process health and auto-restart are useful, but **Herdr is about persistent interactive agent panes, not dev-stack supervision.**" |
| Worktree/review araçları | Conductor, Emdash, Superset | "They are for branch isolation, diffs, and PRs. **Herdr is the live terminal layer those agents can run inside or alongside.**" |
| Web-servis agent | OpenCode web | "OpenCode's web mode serves **one** agent over HTTP; Herdr is a terminal-native multiplexer for **many** agents and harnesses, reachable over plain SSH." |

**Tek satırlık karşılaştırmalar (sayfadan):**
- *"tmux persists terminals; Herdr persists agent workspaces and understands agent state."*
- *"Zellij is a modern terminal workspace; Herdr is an agent multiplexer with state, waits, and orchestration."*
- *"Warp is an agentic development platform; Herdr is the local terminal control layer for your existing agents."*
- *"They orchestrate isolated worktrees and review diffs; Herdr orchestrates live terminals and agent state."*
- Kapanış CTA: *"Start with the multiplexer — If your agents live in terminals, put the terminals in Herdr."*

Karşılaştırma matrisinde herdr'ın "evet" dediği, rakiplerin "hayır" dediği beş sütun:
1. Mevcut terminalinizin içinde çalışır (*"Runs inside your existing terminal"* — rakipler: "no, terminal app" / "no, desktop app" / "no, app workspace")
2. Kalıcı PTY oturum runtime'ı sahiplenir
3. Detach, reattach ve SSH ile bağlanma; tek bir agent'a doğrudan attach
4. Semantik agent durumu (blocked / working / done / idle)
5. Agent-şekilli API (read, send, wait, split, attach)

### A.5 Ürün ilkeleri (yazılı anayasa)

| İlke | Kaynak | Not |
|---|---|---|
| **Mouse-first TUI** | `AGENTS.md` "Herdr is a mouse-first TUI"; `concepts.mdx` "Herdr is mouse-native... keyboard bindings are an optional layer" | Yeni diyalog/onboarding/ayar ekranları mevcut UI dilini yeniden kullanmalı — tek seferlik ekran icat edilmez |
| **Klavye ve mouse ikisi de birinci sınıf** | `README.md:30` "tmux-style prefix keys *and* click, drag, split. **pick per moment, not per tool**" | |
| **Gerçek terminal görüntüsü, yorum değil** | `README.md:27` "real terminal views, **not a wrapped interpretation**"; `docs/next/README.md:17` "you see the agent's own terminal, not someone's interpretation of it" | |
| **Tek Rust binary, Electron yok** | `README.md:32`; `docs/next/README.md:17` "no gui app, no electron, no mac-only native wrapper" | |
| **Hesap yok, telemetri yok** | `website/index.html` "Stable Linux/macOS · Windows preview beta · **no Electron, no account, no telemetry**" | |
| **Opinionated / solo-maintainer ürün** | `CONTRIBUTING.md` "Herdr is opinionated"; "This guide exists so I can keep herdr manageable as a **solo project** and keep it from drifting from what it is supposed to be" | |
| **"You must understand your code"** (Tek Kural) | `CONTRIBUTING.md` — "If you cannot explain what your changes do, how they behave at the edges, and how they fit herdr's existing design, your PR will be closed. **Using AI to write code is fine. Submitting code you do not understand is not.**" | |
| **Ürün yönü tartışılır, PR ile dayatılmaz** | `CONTRIBUTING.md` "If your idea changes or contradicts that direction, do not start with a PR. Start with a discussion." | |

### A.6 Lisans ve iş modeli

- **Çift lisans:** AGPL-3.0-or-later + ticari lisans (`README.md:83-88`, `Cargo.toml:7`). Ticari lisans "AGPL'ye uyamayan organizasyonlar" için.
- **Gelir modeli:** GitHub Sponsors tier'ları (`SPONSORS.md`): Backer $25/ay · Gold $500/ay · Platinum $2.500/ay · Lead Sponsor $5.000/ay · Enterprise custom. Mevcut: 1 Gold (Terminal Trove), 5 Backer, 13 tek seferlik destekçi.
- **Beyan:** *"herdr is independent, open source, and built full-time. Every sponsorship goes directly toward development, stability, and reaching herdr's goal of becoming the runtime for coding agents."*
- **Anlamı:** "Runtime olma" hedefinin **finansmanı açık kaynak sponsorluğuna bağlanmış** — kapalı SaaS'a değil. Bu, blog manifestosundaki "developers own their environments" tezinin iş modeli düzeyinde tutarlı karşılığıdır.

---

## B. MİSYON — bugün fiilen ne yapıyor

### B.1 Kod ölçeği (ölçülmüş)

| Metrik | Değer | Kanıt |
|---|---|---|
| Rust LOC | **247.434** | `find src -name '*.rs' \| xargs cat \| wc -l` |
| Graph node / edge | 24.357 / 129.892 | codebase-memory `index_status` |
| Rust dosya | 298 | `get_architecture` languages |
| Diğer diller | TOML 44 · Python 26 · TypeScript 20 · YAML 19 · Bash 13 · HTML 5 · JS 3 · CSS 2 | `get_architecture` |
| En büyük 5 dosya | `app/input/file_manager.rs` **10.833** · `server/headless.rs` 9.072 · `app/actions.rs` 5.775 · `pane/terminal.rs` 5.293 · `app/mod.rs` 5.180 | `wc -l` sıralaması |
| `#[cfg(test)]` blok | 461 | grep sayımı |
| Agent detection manifest | 19 | `ls src/detect/manifests/` |
| Integration asset seti | 15 agent | `src/integration/assets/` |
| Paket düğüm dağılımı (ilk 15) | app 2897 · codex 1654 · ui 1054 · pane 715 · **fm 635** · server 409 · cli 406 · ghostty 405 · terminal 298 · config 294 · client 292 · platform 273 · api 272 · detect 256 · workspace 236 | `get_architecture` packages |
| Graph hotspot (fan_in) | `ClientControlWriter::clone` 1275 · `TerminalRuntimeRegistry::iter` 864 · `RotatingFileGuard::write` 593 · **`Workspace::test_new` 462** · `RawInputFramer::push` 414 · **`RegionRects::get` 374** | `get_architecture` hotspots |

`Workspace::test_new` fan_in 462 — "PTY'siz test edilebilirlik" ilkesinin nicel kanıtı.

### B.2 Yetenek envanteri — durum ve kanıt

#### B.2.1 UPSTREAM'DEN MİRAS — YAYINLANMIŞ (v0.7.3 stable)

| Yetenek | Durum | Kanıt |
|---|---|---|
| Workspace / tab / pane multiplexing | Yayınlandı | `src/workspace/`, `src/pane/`, `concepts.mdx` |
| Kalıcı PTY + detach/reattach | Yayınlandı | `src/pty/` (actor, backend, fd), `tests/detach_reattach.rs`, `tests/live_handoff.rs` |
| Agent detection (19 agent, hot-reload manifest) | Yayınlandı | `src/detect/manifests/*.toml`: amp, antigravity, claude, cline, codex, cursor, devin, droid, gemini, github-copilot, grok, hermes, kilo, kimi, kiro, maki, opencode, pi, qodercli · `website/agent-detection/` (20 dosya + index.toml = uzaktan dağıtım) |
| Integration asset'leri (agent tarafı lifecycle bildirimi) | Yayınlandı | `src/integration/assets/`: claude, codex, copilot, cursor, devin, droid, hermes, kilo, kimi, mastracode, omp, opencode, pi, qodercli (15) |
| Semantik agent durumu (blocked/working/done/idle/unknown) | Yayınlandı | `SKILL.md:82-91`, `concepts.mdx` tablo. `idle` vs `done` ayrımı = "görüldü mü" attention state'i |
| Socket API + CLI (agent-driven) | Yayınlandı | `src/api/` (272 node), `src/protocol/wire.rs` (PROTOCOL_VERSION), `docs/socket-api.mdx`, `docs/next/api/herdr-api.schema.json`, `herdr api schema` komutu |
| Plugin sistemi | Yayınlandı | `src/app/api/plugins/mod.rs` (3.455 satır), `src/plugin_command.rs`, `src/plugin_paths.rs`, `tests/fixtures/plugin-smoke/` |
| Plugin marketplace | Yayınlandı | `workers/plugin-marketplace/` (Cloudflare Worker + R2), `website/src/pages/plugins.astro` (629 satır), `docs/marketplace.mdx`. GitHub topic `herdr-plugin` ile otomatik, **denetimsiz** index |
| Git worktree yönetimi | Yayınlandı | `src/worktree.rs`, `src/workspace/git/` (config, discovery, status) |
| Uzak/SSH attach | Yayınlandı | `src/remote/`, `docs/persistence-remote.mdx`, `herdr --remote` |
| Windows desteği | **Preview beta** | `src/platform/windows.rs`, `docs/windows-beta.mdx`, `install.ps1`, `scripts/windows_smoke_conpty_path.ps1` |
| Kitty graphics passthrough | Yayınlandı (experimental) | `src/kitty_graphics.rs` — encode/clip/dedup/diff re-emission, 12 test |
| Vendored libghostty-vt | Yayınlandı | `vendor/` + `src/ghostty/` (405 node); patch indeksi `vendor/libghostty-vt.patches.md` |
| Session yönetimi (named sessions) | Yayınlandı | `src/session.rs`, `herdr session list/attach` |
| Copy mode + arama | Yayınlandı | `docs/next/CHANGELOG.md` Unreleased: literal smart-case `/`+`?`, `n`/`N`, tmux-tarzı `w`/`b`/`e` |
| Release kanalları (stable + preview) | Yayınlandı | `website/latest.json` (v0.7.3, protocol 16), `website/preview.json`, `.github/workflows/preview.yml` (Çarşamba/Cuma + manuel) |
| Çok-dilli docs (en/ja/zh-cn) | Yayınlandı | `website/src/content/docs/{ja,zh-cn}/` — 17'şer sayfa; `scripts/docs_translation_parity.py` parity kontrolü |
| Onboarding / release notes / product announcement overlay | Yayınlandı | `src/ui/onboarding.rs`, `src/ui/release_notes.rs`, `src/product_announcements.rs` |

#### B.2.2 CYPACK FORK'A ÖZGÜ — SADECE `feat/native-fm` BRANCH'İNDE (yayınlanmamış)

| Yetenek | Durum | Kod kanıtı |
|---|---|---|
| **Native file manager (Files)** | Fork branch'inde, kapsamlı | `src/fm/` 14 dosya: `miller.rs`, `trail.rs`, `trail_snapshots.rs`, `watcher.rs`, `operations.rs`, `rename.rs`, `delete.rs`, `natsort.rs`, `entry_kind.rs`, `entry_time.rs`, `image_preview.rs`, `text_preview.rs`, `preview_capability.rs`, `mod.rs` (4.928 satır) · `src/ui/file_manager.rs` 3.799 satır + `src/ui/file_manager/` · `src/app/input/file_manager.rs` **10.833 satır** |
| **Shell bölge sistemi** (bounded named regions) | Fork branch'inde | `src/ui/shell/` — `model.rs`, `layout.rs`, `template.rs`, `view.rs`, `interaction.rs`; `RegionRects::get` fan_in **374** (projenin 6. en yoğun sembolü) |
| **Typed Stage / SurfaceHost** | Fork branch'inde | `src/ui/surface_host.rs` — `enum StageSurfaceView` (satır 43), `enum LaunchPolicy` (satır 26), `struct StageState` (satır 77); `StageState::surface_view` fan_in **47** |
| **AppDock** | Fork branch'inde | `src/ui/app_dock.rs`; preferred 5, min 3, max 9 hücre; sağ-tık isim popover'ı |
| **Shell input router** | Fork branch'inde | `src/app/input/shell.rs::route_shell_input(ShellInputRouteContext) -> ShellInputOwner`; docstring: *"closed instead of leaking to a hidden background surface"* |
| Miller sütun görünümü + yatay viewport | Fork branch'inde | `src/fm/miller.rs`, `src/fm/trail.rs`; bounds: chain ≤32 segment, resident ≤5 sütun, görünür ≤5 tam sütun |
| Miller sütun resize | Fork branch'inde | min 16 / preferred 28 / max 64 hücre; paylaşılan `ResizeTransaction` (ayrı drag state YOK) |
| Metin preview | Fork branch'inde | `src/fm/text_preview.rs` 307 satır; arka planda, input loop dışında |
| Görsel (image) preview | Fork branch'inde | `src/fm/image_preview.rs` 734 satır; Path β (native local placement) |
| Dosya işlemleri | Fork branch'inde | `src/fm/operations.rs` (copy/move), `rename.rs` (tekil + bulk), `delete.rs` (trash + kalıcı); `src/app/file_operation_worker.rs` 3.722 satır (bounded progress, generation-safe cancellation) |
| Dosya→agent handoff | Fork branch'inde | `src/app/file_agent_handoff.rs`, `src/app/agent_reference_picker.rs`; **no-submit invariant** (CR/LF/Enter yok) |
| Filesystem watcher | Fork branch'inde | `src/fm/watcher.rs`, `src/app/file_manager_watcher.rs`; deterministik reconciliation |
| Locations/Favorites rail | Fork branch'inde | `src/app/file_manager_locations.rs`, `file_manager_locations_model.rs`; Wide/Standard/Compact projeksiyon |
| Bounded FM I/O worker | Fork branch'inde | `src/app/file_manager_io_worker.rs`; one-running/one-latest, render/input bloklamaz |
| Preview worker'ları | Fork branch'inde | `src/app/file_preview_worker.rs`, `src/app/image_preview_worker.rs` |
| Playwright görsel test altyapısı | Fork branch'inde | `tests/visual/` — 8 spec (`trail`, `focus`, `navigation`, `icons`, `mtime-groups`, `mutation`, `picker`, `fractional-scroll`, `files-locations`, `harness`) + `harness/grid.js`; VIS-01..VIS-27 baseline |
| Deterministik görsel fixture exporter | Fork branch'inde | `src/ui/visual_fixture.rs` (1.250+ satır); A/B run + SHA-256 diff |
| Snapshot v4 shell persistence | Fork branch'inde | `src/persist/`; v3 sidebar width migration, geçersiz shell verisi containment, ileri sürüm reddi |
| Render profiling | Fork branch'inde | `src/render_prof.rs`; `HERDR_RENDER_PROF=1` |

#### B.2.3 PLAN AŞAMASINDA / AÇIK

| Öğe | Durum |
|---|---|
| Change Pipeline (T3–T10) | Duraklatılmış (25/114 madde) |
| Custom Layout B-zinciri (B1–B4) | Tasarım öncesi, **hiç başlamadı** |
| Belge (PDF/XLSX/DOCX) render | Sadece metadata; plugin yolu tasarlandı, uygulanmadı (FMR-5 P5) |
| Dosya İÇERİK düzenleme | **Yok** — hiçbir kod yolu bulunamadı |
| FMN-6 dizin pre-warm | Yetkisiz (RED gerekli) |
| FMR-0 scroll versiyon sıralaması | 4 versiyon toplandı, matris çalıştırılmadı |
| S5 ComponentRegistry, S7 popup stack | Kalıcı NO-GO |
| M3 genel UI panel/sayfa/buton arayüzü | Kanıtla NO-GO |

---

## C. MİMARİ ANAYASA VE KODDA TUTULMA DOĞRULAMASI

`AGENTS.md` (= `CLAUDE.md` symlink, 16.838 byte) yedi ilke tanımlıyor. Her biri graph + grep ile doğrulandı:

| # | İlke (AGENTS.md beyanı) | Kod kanıtı | Durum |
|---|---|---|---|
| 1 | **"State is separated from runtime."** `AppState` saf veri, PTY/async olmadan test edilebilir; `PaneState` ≠ `PaneRuntime` | `src/pane/state.rs:6 PaneState` vs `src/pane.rs:905 PaneRuntime` — ayrı dosya, ayrı tip. `AppState::test_new()` + `Workspace::test_new()` (fan_in **462**) PTY'siz test kanıtı | ✅ TUTULUYOR (0.9) |
| 2 | **"Render is pure."** `compute_view()` geometri ve mutasyonu üstlenir; `render()` yalnız `&AppState` alır ve çizer | `src/ui.rs:150 compute_view(app: &mut AppState, area: Rect)` · `src/ui.rs:779 render(app: &AppState, frame: &mut Frame)` — **imza düzeyinde zorlanmış**: render immutable referans alıyor, mutasyon derleme zamanında imkânsız. `src/ui/compose.rs:52` docstring: *"Paint into `area`. **Pure: reads `ctx`, never mutates state.**"* | ✅ TUTULUYOR (**0.95**) — dilin tip sistemi ilkeyi garanti ediyor |
| 3 | **"No god objects."** Bir modül çok iş yapıyorsa bölünmeli; `app/` state/actions/input olarak bölünmüş, öyle kalmalı | `src/app/` 30+ dosya (state, actions, input/, api/, runtime, session, projects, worktrees, creation, preview, agents, ids, theme_sync...). AMA: `app/input/file_manager.rs` **10.833 satır**, `app/mod.rs` 5.180, `app/actions.rs` 5.775 | ⚠️ KISMEN (0.6) — bölünme var ama dosya düzeyinde tanrı-nesne baskısı fork tarafında birikmiş |
| 4 | **"Platform code is isolated."** OS davranışı `src/platform/<os>.rs`'de; core modüllerde `#[cfg(target_os)]` yok | `src/platform/{linux,macos,windows,fallback,mod}.rs` var (6 `cfg(target_os)` içeride). AMA platform/ **dışında 12 kullanım, 8 dosyada**: `sound.rs`, `app/projects.rs`, `app/input/mod.rs`, `app/preview.rs`, `server/autodetect.rs`, `ui/visual_fixture.rs`, `detect/mod.rs`, `pane/osc.rs`. `cfg(unix)`/`cfg(windows)` 512 kullanım (ilke bunlara izinli) | ⚠️ KISMEN (0.85) — `cfg(target_os)` sızıntısı ilkenin lafzına aykırı |
| 5 | **"Detection is decoupled."** Detector ekran snapshot'ı okur, parser veya viewport state'ine dokunmaz | `src/detect/` ayrı paket (256 node); manifest TOML'lar saf veri; `herdr agent read --source detection` ayrı okuma kaynağı (kullanıcı viewport'undan bağımsız — kullanıcı scroll edebilir) | ✅ TUTULUYOR (0.85) |
| 6 | **"Screen detection is evidence-based."** Manifest değişikliği önce canlı pane okuması ister; invariant/alternatif kontroller açık AND/OR gate olarak kodlanır | `AGENTS.md` "Agent Detection Updates" bölümü hot-reload döngüsünü tanımlıyor (`herdr agent read` → `herdr agent explain --json` → manifest → `herdr server reload-agent-manifests`); `scripts/agent_detection_manifest_check.py` + `test_agent_detection_manifest_check.py` | ✅ TUTULUYOR (0.85) |
| 7 | **"UI patterns should be reused."** Yeni dialog/onboarding/settings/post-update akışları mevcut UI/UX dilini kullanmalı | Paylaşımlı `render_modal_choice_list<T>`, `ScrollMetrics` scrollbar, `centered_popup_rect`, `src/ui/widgets.rs`; `.codex/MEMORY.md`: *"Do not add an arbitrary ComponentRegistry before its separate S5 trigger is proven"* | ✅ TUTULUYOR (0.8) |

### C.1 Runtime/client sınır guardrail'i — en kritik mimari kural

`AGENTS.md` "Runtime/client boundary guardrail" bölümü, herdr'ın **server-owned runtime protokolü + TUI'nin yalnızca bir istemci olması** yönündeki göçünü koruyor:

> "Herdr is migrating toward a server-owned runtime protocol with the TUI as one client. New work should not deepen the current server/TUI coupling."
> "Do not add new shared behavior that only works through the private TUI client socket. **Use neutral server/API names, not UI-surface names like sidebar, row, card, or widget.**"

**Sınıflandırma tablosu (AGENTS.md'den birebir):**
- **Server/runtime:** Pane/agent metadata, process state, terminal state, events
- **TUI/client:** Sidebar layout, token yerleşimi, renkler, seçim, modal, mouse/viewport state
- **Ortak (şimdilik):** Workspace/tab/pane oturum organizasyonu — ama ilgisiz runtime özellikleri için zorunlu kimlik yapılmamalı

**Fork'un bu kurala uyumu — kanıtlı:**

Fork'un tüm Stage/Shell/Files işi bilinçli olarak **client-local** tutulmuş:

1. `.codex/MEMORY.md`: *"Typed Stage identity is TUI/client-local presentation state. **It must not own, create, destroy, or rename server/runtime terminal identity.**"*
2. SF4.1 görev tanımı (`.codex/TASKS.md`): *"Keep the new identity client-local; **add no server, protocol, pane, tab, workspace, or terminal identity.**"*
3. SF4.1-08 test: `stage_surface_switch_does_not_destroy_terminal_runtime` — Stage geçişinin terminal runtime sayısını/kimliğini bozmadığını kanıtlıyor
4. FFO kapanış kanıtı (`.codex/CURRENT.md:75-77`): *"The FFO diff is **empty** in `src/server`, `src/protocol`, `src/platform`, `Cargo.toml`, and `Cargo.lock`, and adds no production filesystem read, worker, cache, channel, timer, sleep, or debounce."*

Bu, forkun 819 commit'lik divergence'a rağmen **upstream'in mimari anayasasını ihlal etmediğinin** en güçlü kanıtıdır. Güven: 0.9 (belge kanıtı; diff bağımsız çalıştırılmadı).

### C.2 Kod hijyeni ilkeleri

| İlke | Ölçüm | Değerlendirme |
|---|---|---|
| "no `unwrap()` in production code" | `src/` içinde toplam 3.344 `.unwrap()`; kaba test-dışı filtre 2.688 | ⚠️ **Ham sayı yanıltıcı.** 461 `#[cfg(test)]` bloğu var ve testler production dosyalarının İÇİNDE yaşıyor (`#[cfg(test)] mod tests` konvansiyonu). Kaba grep test bloklarını ayıramıyor. Proje bunun yerine **"added-production-unwrap" diff denetimi** kullanıyor (`.codex/TASKS.md` N2.1c: *"production-unwrap/diff/artifact scans"*). Yöntem sağlam; mutlak sayıdan ihlal çıkarılamaz. **Confidence 0.5 — bağımsız doğrulanamadı** |
| Test invariant altyapısı | `AppState::assert_invariants_for_test` (complexity 35, cognitive 71, self-recursive) + `AppState::test_with_adversarial_identity_state`; `Workspace::assert_invariants_for_test` + `Workspace::test_adversarial_identity_state` | ✅ Graph'ta doğrulandı — kimlik/state refactor'ları için adversarial test kancası gerçekten mevcut ve AGENTS.md'nin "refactor-risk" protokolüyle uyumlu |
| `clippy.toml` | `too-many-arguments-threshold = 11` | Tek özel kural — geri kalan varsayılan Clippy |
| Gate seti | fmt · cargo nextest · Linux all-target Clippy · Windows MSVC Clippy · Python maintenance (68 test) · Bun (5+12) · Playwright Chromium (35) | `justfile` + `.codex` kanıt dosyaları |

---

## D. FAZ HARİTASI VE TAMAMLANMA ORANLARI

### D.1 Genel tamamlanma sayımı

| Kayıt | Kapalı | Açık | Toplam | Oran |
|---|---|---|---|---|
| `.codex/TASKS.md` (ürün) | **468** | 24 | 492 | **%95,1** |
| `.codex/CHANGE-PIPELINE-TASKS.md` (araç lane) | **25** | 89 | 114 | **%21,9** |
| **Birleşik** | 493 | 113 | 606 | **%81,4** |

### D.2 Shell Foundation zinciri (SF0–SF6) — 7/7 KAPALI (%100)

| Faz | Amaç | Durum | Kapanış kanıtı |
|---|---|---|---|
| **SF0** | Tasarım + baseline dondurma; A0-A7 boyut analizi; 7 Foundation + 5 FM fazına açık kullanıcı onayı | ✅ | Artifact commit `32856f7`, iki CyPack ref'te SHA eşitliği; graph 19.808/91.543; `.codex/evidence/shell-foundation-plan-review.md` |
| **SF1** | Karakterizasyon testleri (I6); mevcut curtain davranışının dondurulması | ✅ | `7b9b626d` "test: characterize shell foundation baseline"; focused 11/11, full nextest 3203/3203, Bun 17/17, Python 64/64, graph 19.809/91.610 |
| **SF2** | Bounded named-region model + typed template + deterministik solver + cached `ShellView` | ✅ | SF2.1-2.3 → `f272a881`; SF2.4 RED/GREEN `2a440478`/`07133b8b`; kapanış: cached-view 7/7, broad shell 88/88, `src/ui.rs` 41/41, full 3239/3239, graph 20.017/91.917 |
| **SF3** | Resize / collapse / scroll / snapshot-v4 persistence | ✅ | SF3.1 divider reducer `368c4d3a`/`d89a7f94`, `b6570ee4`/`807cb76c`, `61b915a9`, `336fa3de` (8/8 keyboard, 119/119 broad, full 3264/3264) · SF3.2 `45a2e87e` (scroll 6/6, broad 202/202, full 3281/3281) · SF3.3 `90be6893` (snapshot matrisi 12/12, broad 137/137, full 3292/3292, graph 20.291/94.542) |
| **SF4** | SurfaceHost + focus/input router + overlay bloklama + saf render projeksiyonu | ✅ | **SF4.1** 8/8 slice GREEN (`557bcc77`/`6a18f0c7` … `784fdc2e`/`944a9d4c`), full 3300/3300, graph 20.396/93.372 · **SF4.2** 8/8 slice, head `20f659c1`, Rust 3.309/3.309 · **SF4.3** SF4.2 içinde teslim · **SF4.4** 6/6 GREEN, head `f973740e`, Rust 3.315/3.315 |
| **SF5** | AppDock (icon-only Terminal/Files, preferred 5 / min 3 / max 9) | ✅ | SF5.1 `64d5dd5e`/`cb0c77fd`; SF5.2 `406db487`/`d031ef26`; paylaşılan `ResizeTransaction` dock 3..=9 için pinned |
| **SF6** | Files'ı native Workspace Stage yapmak (terminal curtain kaldırma), tüm FM semantiğini koruyarak | ✅ | `.codex/evidence/shell-foundation-sf6-files-stage-progress.md` |

### D.3 File Manager zinciri (FM1–FM5) — 5/5 KAPALI (%100)

| Faz | Amaç | Durum | Kanıt |
|---|---|---|---|
| **FM1** | Yatay Miller viewport (logical history ≤32, resident ≤5, görünür ≤5 tam sütun) | ✅ | `35cfbc00` production compute caller; `.codex/evidence/fm1-miller-viewport-progress.md` |
| **FM2** | Miller sütun resize (min16/pref28/max64), ayrı `MillerTrioDrag` yerine paylaşılan `ResizeTransaction` | ✅ | Preview sırasında sıfır persistence/PTY/filesystem/image-target churn; 1.000-move sınırı |
| **FM3** | Tüm-sütun mouse sahipliği (generation-checked, `ConsumedStale` sözleşmesi) | ✅ | Ctrl/Shift operasyon yetkisi current-directory-only kalır |
| **FM4** | Finder-benzeri path-stable büyüyen navigasyon | ✅ | N2.1 testleri korunur; chain ≤32, resident ≤5; 10.000-action adversarial invariant |
| **FM5** | Preview/Inspector yerleşimi — ölçüm + açık GO/NO-GO | ✅ **NO-GO** | `.codex/evidence/fm5-preview-placement-decision.md:11-12` — *"NO-GO for Shell RightPanel and adaptive hybrid product work. **Keep the existing inline final Miller preview column.**"* |

### D.4 Post-FM iyileştirme dalgaları (kronolojik)

| Program | Konu | Durum | Kapanış kanıtı | Açık kalan |
|---|---|---|---|---|
| **FIP** (Files Interaction Polish) | FIP-G, FIP-0..6; agent'a referans teslimi, no-submit invariant | **%87,5** (FIP-6'da 5/8) | `.codex/evidence/fip-baseline-freeze.md`, `fip-progress.md` | **FIP-6.3** (izole terminal mouse + PTY-byte smoke), **FIP-6.7** (continuity), **FIP-6.8** (git ancestry/push) |
| **TRAIL** | Miller Trail kanonik UX kontratı | ✅ Kapalı | `docs/superpowers/specs/2026-07-18-herdr-miller-trail-ux-contract.md` | — |
| **FMR** (Visibility/Preview/Plugin/Mouse) | FMR-0..5 | **~%70** | FMR-1 `b385ca3a`/`de136da5` (VIS-13, görsel suite 21/21) · FMR-2 `0b69b557`/`0b8ab32f`/`918ae4df` · FMR-2A `dbfa55be`/`72cdce83` · FMR-3 `4c87a18f`/`ea75a269`/`b79b55f6` (VIS-14) | **FMR-0** (4-versiyon scroll lab matrisi + sıralama), **FMR-4** (bağımlılık lisans/güvenlik re-doğrulama), **FMR-5** (P5 plugin adapter → P6 gates → P7 ranking) |
| **MTIME** | mtime sıralama + Finder-benzeri gruplar | ✅ Kapalı | `.codex/evidence/miller-mtime-groups-closure.md`, `miller-mtime-dependency-audit.md` | — |
| **FCL** (Files Content Locations Rail) | FCL-0..7; Files-local rail + bounded I/O worker + exact origin | ✅ Kapalı | FCL-0 `fbc2c78d` · FCL-1 `96958b86`/`ce56e6ef` + `0753999d`/`4e5ee9b0`/`249e8315` · FCL-2 `1e5c3927`/`c814a72b`/`2960c053`/`c0358812`/`4755fdaf`/`b12ef4be`/`5ecd0159` · FCL-3 `c052b8d4`/`8c75f989` · FCL-4 `8c6cfd5c`/`b98ed2ac` | — |
| **FMP** (Files Mouse/Nav Performance) | Rapid-navigation latency kök nedeni | ✅ Kapalı | `.codex/evidence/files-rapid-navigation-root-cause.md`, `files-rapid-navigation-scale-calibration.md`; final residual: `b2accbb4`, `8851b5e0`, `ed329058`, `d8583d3a` | — |
| **FMN** (Movement Semantics + Wheel) | FMN-0..6; cursor-only hareket vs explicit aktivasyon; wheel burst normalizasyonu | **FMN-0..5 kapalı, kullanıcı fiziksel kabulü ALINDI** | Yayın başı `787bb96b`; ham Ghostty kanıtı 333 dikey event / 226 same-direction delta <2ms (identical-coordinate triplet/sextuplet), sonraki grup ≥5ms; focused 302/302, full 3.619/3.619+4 skip, Chromium 33/33, exporter 1/1 | **FMN-6** (Home/Desktop/Downloads pre-warm — ölçüm-first, yetkisiz) |
| **FMH** (Horizontal Miller Focus) | Right/`l` yalnızca dizinde ilerler, dosyada `Inert` | ✅ Kapalı (tarihsel) | Behavioral RED `0ddfe67c-...`; FMH 3/3, cross-layer 10/10, broad FM 190/190, full 3.622/3.622+4 skip, Chromium 33/33, sıfır JSON/PNG delta | — |
| **FFO** (Files Focus Ownership) | Rail/Trail tek üst-seviye sahip sözleşmesi | Otomatik kapanış TAMAM | 10 commit zinciri: `bf9fcf46` design → `0e415d81` plan → `0549c8aa` karakterizasyon → `3c5f94e4`/`6b18529a` input → `83fb77ec`/`de6656e5` action → `680eb194`/`4422f8ae` visual → `d85d610e` oracle; full 3.680/3.680+6 skip, Chromium 35/35, exporter A/B 1/1 boş diff, graph 24.327/129.874 | **FFO-8** (exact stage + docs commit + CyPack push), **FFO-9** (kullanıcı fiziksel `TP-FFO-E2E-01`) |
| **DCLICK** (Directory Primary-Click Focus) | Klik = tam satır odağı; aktivasyon yalnız Right/`l`/Enter | Yayınlandı | RED `da413d1d` (run `1fcd96df-...`, 0/2 fail at `active_col 1 != 0`) → GREEN `b90a177d` (run `3f217ee8-...`, 2/2) → docs `f14c112e` → tip `b48bd903`; old-contract audit 141/144 → dönüşüm sonrası 145/145; focused 8/8, broad file_manager 307/307, full 3.683/3.683+6 skip, Chromium 35/35, graph 24.357 | **DCLICK-6** (kullanıcı fiziksel izole E2E) |

### D.5 Mission Roadmap (M1–M3)

| Faz | Amaç | Durum | Kanıt |
|---|---|---|---|
| **M1** | Focused-Agent Attachment Picker | ✅ Kapalı | `.codex/evidence/m1-agent-attachment-picker.md` |
| **M2** | Git Worktree Management Actions | ✅ Kapalı | `.codex/evidence/m2-worktree-management-actions.md` |
| **M3** | Genel Panel/Sayfa/Buton arayüz değerlendirmesi | ❌ **Kanıtla NO-GO** | `.codex/evidence/m3-general-ui-interface.md`; M3.0 final decision |

### D.6 Deferred UI Architecture (P4)

| Aday | Mevcut kanıt | Eksik tetik | Karar |
|---|---|---|---|
| **S5 ComponentRegistry** | `Compositor` iki sabit `Component` katmanı içeriyor; `BaseLayer` tek açık terminal/FM içerik takası yapıyor; dinamik kayıt / per-component event ownership / ikinci sayfa lifecycle YOK | Render, hit-area, lifecycle ve event routing'i çoğaltan **ikinci gerçek bağımsız component/page** | **Implementation NO-GO** — somut içerik-takas deseni korunur |
| **S6 Resizable persisted shell** | `ShellLayout::default()` yalnız LeftPanel/CenterContent hesaplıyordu; `SessionSnapshot` somut sidebar width/split persist ediyordu ama shell tree yoktu | Gerçek RightPanel/BottomBar tüketicisi | **Süperseded** → SF0-SF6'ya absorbe edildi (gerçek kullanıcı talebi geldi) |
| **S7 Popup ownership stack** | Tek `Mode` tek `OverlayLayer` seçiyor; `render_modal_shell` 8 çağıran, `modal_stack_areas` 10; context/modal transition testleri focus/close order'ı zaten koruyor | Gerçek **eşzamanlı iç içe popup** | **Implementation NO-GO** — mevcut modal/context seam'leri yeniden kullanılır |
| **N2 Dynamic/unbounded Miller** | V1 A-C kapalı; `FmState::enter/leave/reload` cached parent/current/preview yeniliyor; pinned Yazi/Joshuto kanıtı yalnız "departed-child focus"u eksik gösteriyor | Keyfi görünür zincirin tetiği yok | **Dynamic NO-GO**; yalnız **N2.1 path-stable parent return GO** (`e433a2f` RED → `c530836` GREEN, 6/6 + fm::tests 65/65, full 3177/3177) |
| **N2.2** | — | — | **Süperseded** → FM1-FM4'e absorbe (32-segment/5-resident sınırlı) |

### D.7 Change Pipeline (T0–T10) — %21,9, DURAKLATILDI

| Blok | Konu | Durum |
|---|---|---|
| **T0** | Tasarım, governance, mid-flight adoption sözleşmesi | ✅ |
| **T1** | Kod-seviyesi TDD implementation planı | ✅ |
| **T2** | Ratatui Design Intelligence v2.1 | ✅ (`86a25e8`) |
| **T3** | `herdr-change-pipeline` modül scaffold | ⏸ **T3.1'de duraklatıldı** |
| **T4** | A0-A7 change-intelligence engine | ⏸ |
| **T5** | I0-I14 delivery engine | ⏸ |
| **T6** | Cross-test aileleri | ⏸ |
| **T7** | Adapter'lar + senaryo fixture'ları | ⏸ |
| **T8** | Modül/repo doğrulama | ⏸ |
| **T9** | Git publication + codebase memory | ⏸ |
| **T10** | Pilot, ders, kapanış | ⏸ |

Duraklatma gerekçesi (`.codex/MEMORY.md`): *"The non-product change-pipeline lane is paused at T3.1 until the current sequential product phase closes. **Product and tooling commits never mix.**"*

### D.8 Custom Layout B-zinciri (B1–B4) — %0, HİÇ BAŞLAMADI

| Adım | Tanım | Durum |
|---|---|---|
| **B1** | Keşif: mockup bölgeleri ↔ `ShellLayout/AppDock/Stage` seam eşlemesi; `custom-layout-SYSTEM-MAP.json` üretimi | ❌ Başlamadı |
| **B2** | Design spec (`docs/superpowers/specs/…custom-layout-design.md`): bölge sözleşmeleri, runtime/client sınıflandırması, no-goals | ❌ **Dosya YOK** (doğrulandı) |
| **B3** | Implementation plan: RED adları + beklenen fail'ler + GREEN seam'leri + görsel VIS-ID'leri | ❌ Yok |
| **B4** | Katman-katman yürütme (test noktaları → RED → GREEN → Playwright baseline → gates → continuity → FF push) | ❌ Yok |

---

## E. VİZYON ↔ GERÇEKLİK BOŞLUK GRID'İ

```
  ── herdr vizyon ↔ gerçeklik · 2026-07-24 ──  (SOL: beyan edilen vizyon: 18 · SAĞ: kod/branch gerçeği: 18)
┌────┬──────────────────────────────────────────────┬────┬──────────────────────────────────────────────────┐
│ #  │ 📜 BEYAN EDİLEN VİZYON / HEDEF               │ ⟷ │ 🦀 KODDA / BRANCH'TE GERÇEKLEŞEN                 │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 1  │ "agent multiplexer in your terminal"         │ ✅ │ workspace/tab/pane + PTY + detach — yayında      │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 2  │ semantik agent durumu blocked/working/done   │ ✅ │ 19 manifest + 15 integration asset               │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 3  │ "agents can use herdr too" — socket API      │ ✅ │ src/api/, protocol v16, SKILL.md, agent-guide    │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 4  │ "mouse and keyboard both first-class"        │ ✅ │ input/mouse.rs 4.372 satır + route_shell_input   │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 5  │ "one rust binary, no electron"               │ ✅ │ tek crate herdr 0.7.3, 247k LOC Rust             │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 6  │ "render is pure" (mimari ilke)               │ ✅ │ render(&AppState) imzayla zorlanmış — ui.rs:779  │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 7  │ "state separated from runtime"               │ ✅ │ PaneState(pane/state.rs) ≠ PaneRuntime(pane.rs)  │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 8  │ runtime/client boundary — TUI yalnız istemci │ ✅ │ Stage/Shell %100 client-local; server diff BOŞ   │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 9  │ "platform code is isolated"                  │ ⚠️ │ platform/ var AMA cfg(target_os) 8 dosyada sızmış│
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 10 │ "no god objects" — app/ bölünmüş kalmalı     │ ⚠️ │ bölünmüş AMA input/file_manager.rs 10.833 satır  │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 11 │ "becoming the runtime for coding agents"     │ ❓ │ multiplexer+API var; "runtime" hedefi YOLDA      │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 12 │ per-agent support matrix yayını (blog sözü)  │ ❓ │ manifest'ler var, PUBLIC matris sayfası YOK      │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 13 │ FORK: "yazi/superfile'ı aşan file manager"   │ ⚠️ │ src/fm/ 14 dosya + 14k satır input — YAYINSIZ    │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 14 │ FORK: custom layout altyapısı (7 bölge)      │ ❓ │ shell/ seam'leri HAZIR; B1-B4 zinciri BAŞLAMADI  │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 15 │ FORK: RightPanel/BottomBar gerçek tüketici   │ ❓ │ fixture var, GERÇEK tüketici yok (S6 trigger)    │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 16 │ PNG/görsel önizleme (image preview)          │ ✅ │ src/fm/image_preview.rs 734 satır (B0/B2 kapalı) │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 17 │ PDF/XLSX/DOCX RENDER                         │ ❌ │ SADECE metadata: DocumentMetadata reason         │
├────┼──────────────────────────────────────────────┼────┼──────────────────────────────────────────────────┤
│ 18 │ Dosya İÇERİK düzenleme (edit)                │ ❌ │ HİÇBİR kod yolu yok — sadece rename/copy/delete  │
└────┴──────────────────────────────────────────────┴────┴──────────────────────────────────────────────────┘
  Açıklama: ✅ vizyon kodda karşılanmış  ⚠️ kısmen/kaymış (ihlal sinyali veya yayınlanmamış)
            ❓ yalnızca planda/beyanda — kodda karşılığı yok  ❌ eksik, aksiyon gerekli
```

**Grid okuma notu:** En kritik satırlar 13, 17, 18. Satır 13 fork'un birincil hedefinin **teknik olarak büyük ölçüde gerçekleştiğini ama hiçbir kullanıcıya ulaşmadığını** gösteriyor. Satır 17-18 kullanıcının şimdi odaklanacağı iki özelliğin **sıfır kod tabanına** sahip olduğunu. Satır 9-10 mimari ilkelerin kod büyüdükçe aşınma sinyalini veriyor.

---

## F. STRATEJİK RİSKLER VE AÇIK KARARLAR

### R1 — Fork/upstream divergence (EN YÜKSEK RİSK)

**Ölçüm:** §0.2'deki tablo (819 commit toplam divergence, merge-base 2026-07-11, %100 CyPack yazarlı).

**Risk:** 819 commit'in **hiçbiri upstream'e gitmedi ve yapısal olarak gidemez**:

- Aktif hesap `CyPack` ≠ `ogulcancelik` → external contributor kuralları geçerli. `.codex/MEMORY.md`: *"The acting GitHub account is `CyPack`; this is external-contributor/fork work. **Never push upstream or open upstream issues/PRs for the user.**"*
- `CONTRIBUTING.md`: İlk PR'dan önce kabul edilmiş issue + maintainer `/approve @username` şart (`.github/APPROVED_CONTRIBUTORS`)
- Feature request'ler, fikirler, sorular ve katkı önerileri **Discussion'a** yönlendirilir, issue'ya değil
- *"If a PR introduces a feature without prior alignment, or changes herdr's feel without discussion, it will likely be closed."*
- *"Bigger changes to UI, behavior, interaction patterns, persistence, or architecture need discussion and maintainer approval first."* — **fork'un yaptığı tam olarak budur** (Shell/Stage/AppDock/Files = mimari değişiklik)
- `AGENTS.md` ayrıca AI ajanlarının kullanıcı adına issue açmasını **yasaklıyor**: *"Do not use the GitHub CLI, API, browser automation, or any other tool to submit an issue on their behalf."*

**Landing hedefi (fiili):** `.codex/BOOTSTRAP.md:29` + `.codex/MEMORY.md` — *"CyPack fork-only fast-forward pushes"*; `.codex/CHANGE-PIPELINE-TASKS.md` global constraints: *"**No push to `upstream`, no force push**, and no unrelated staging."* Tüm iş yalnızca CyPack fork'unun `master` + `feat/native-fm` ref'lerine iniyor.

**Sonuç:** herdr fork'u fiilen **bağımsız bir türev ürün** haline geldi ama kendi dağıtım kanalı (release, install script, doküman sitesi) yok. Bu R2'yi doğuruyor.

### R2 — Yayınlanmamış özellik birikimi

- `Cargo.toml` version hâlâ `0.7.3` — upstream'in son stable'ı; fork kendi sürüm numarasını almamış
- `website/latest.json` upstream v0.7.3'ü gösteriyor, asset URL'leri `github.com/ogulcancelik/herdr/releases/download/v0.7.3/...`
- Fork'un release workflow'u yok; `.github/workflows/release.yml` upstream'in
- Kurulu binary ile geliştirme binary'si farklı: `.codex/evidence/files-visibility-preview-plugin-research.md` — *"The installed binary `/home/ayaz/.local/bin/herdr` is dated 2026-07-12... **Running plain `herdr` selects the older installed binary; the new scroll and sidebar behavior exists only in the current debug build** until a separately authorized install/release workflow updates the installed binary."*

**Etki:** Kullanıcı kendi geliştirdiği file manager'ı günlük kullanamıyor; her test izole debug build gerektiriyor (`.local/ISOLATED-DEV-TEST.md`, `.local/herdr-trail-test.sh`, `.local/herdr-files-v1-profile.sh`).

### R3 — Upstream ile senkron kaybı riski

Fork upstream'in 4 commit'ini almamış. Şu an küçük ama merge-base 2026-07-11'de sabit — her geçen gün upstream ilerledikçe rebase/merge maliyeti artıyor. Özellikle `749e85e0 fix: detach windows server from host terminal` gibi platform düzeltmeleri kaçırılıyor.

### R4 — Doküman/süreklilik katmanının çoğu git dışı

`.gitignore`: `/docs/*` + `!/docs/next/` + `!/docs/next/**` ve `/.local/` ve `.cartography/`.

Ölçüm: `docs/` altında 94 dosyanın **87'si takipte, 7'si takipsiz**:

| Takipsiz dosya | İçerik |
|---|---|
| `docs/patterns/native-file-manager.md` | P1-P7 FM pattern kataloğu |
| `docs/patterns/rust-engineering.md` | Rust mühendislik pattern'leri |
| `docs/patterns/tui-composition.md` | TUI kompozisyon sentezi (P1-P4) |
| `docs/references/README.md` | Domain-indexed referans havuzu |
| `docs/references/native-file-manager.md` | FM dış kaynak kayıtları |
| `docs/references/tui-composition.md` | TUI kompozisyon kaynakları |
| `docs/superpowers/specs/2026-07-17-herdr-custom-layout-architecture-guide.md` | **Custom layout mimari rehberi** |

Ayrıca `.cartography/` (7 SYSTEM-MAP: files-content-locations-rail, files-rapid-navigation-latency, fip-closure, herdr-fm-capability, plus-latency, rust-engineering, tui-composition) ve `.local/` tümü (prd/, session handoff'lar, evidence, profil script'leri) takipsiz.

**`reference-registry` kuralının gerektirdiği kalıcı bilgi katmanı git korumasında değil.** Disk kaybı = kurumsal hafıza kaybı. **Bu dosyanın kendisi de aynı durumda** (`docs/analysis/` gitignored) — bu yüzden frontmatter'da makine kopyası yolu (`~/.cartography/herdr-vision-mission-*`) belirtildi.

### R5 — Belge/kod senkron kayması

`docs/superpowers/specs/2026-07-17-herdr-custom-layout-architecture-guide.md:136-144` "Status against the target" tablosu hâlâ:

| Step | State (belgede yazan) | Gerçek |
|---|---|---|
| SF4 typed surface foundations | CLOSED (`f973740e`) | ✅ doğru |
| SF5.1 dock model/geometry/render | **NEXT** | ❌ KAPALI |
| SF5.2 dock interaction/popover | **pending** | ❌ KAPALI |
| SF6.1-6.3 Files-to-Stage migration | **pending** | ❌ KAPALI |
| FM1.1-1.3 horizontal Miller viewport | **pending** | ❌ KAPALI |
| FM2.1-2.2 column edge drag-resize | **pending** | ❌ KAPALI |
| FM3+ all-column mouse, growing navigation | **pending** | ❌ KAPALI |

Belge 2026-07-17'de donmuş, 7 gün bayat. Kendini "authority order on conflict: the plans win" diyerek ikincil ilan ediyor ama yine de yanlış durum bilgisi taşıyor ve gelecek ajanı yanıltabilir.

### R6 — Modül şişmesi ("no god objects" ilkesine baskı)

`src/app/input/file_manager.rs` 10.833 satır — projedeki en büyük dosya, ikincinin (`server/headless.rs` 9.072) üstünde. AGENTS.md: *"If a module is doing too many things, split it."* Bu, gelecekteki custom layout işinin **refactor-risk** sınıfına gireceğini gösteriyor (AGENTS.md: iki+ core yüzeye dokunan, persisted state / protokol ID / workspace-tab-pane kimliği / restore-handoff / agent detection authority / UI-input state projeksiyonu değiştiren değişiklikler → karakterizasyon testi + roundtable şart).

### R7 — Kullanıcı onayı bekleyen kararlar (açık)

| # | Karar | Kaynak | Not |
|---|---|---|---|
| 1 | **DCLICK-6** fiziksel izole E2E kabulü | `.codex/TASKS.md` P0 ACTIVE | Reçete: `cd /home/ayaz/projects/herdr && HERDR_RENDER_PROF=1 ./.local/herdr-trail-test.sh run` |
| 2 | **FFO-9** `TP-FFO-E2E-01` fiziksel kabul | `.codex/TASKS.md` P0 ACTIVE | Rail/Trail mouse-to-key ownership, tek adım wheel, Right/Left kolon yasaları, Rail-disabled aksiyonlar, tek dolu aktif satır |
| 3 | **FMN-6** Home/Desktop/Downloads pre-warm | `.codex/MEMORY.md` | *"not authorized without a separate first-entry latency RED"*; genel/sınırsız LRU asla |
| 4 | **FMR-0** 4 scroll versiyonundan production adayı seçimi | `.codex/TASKS.md` | *"Rank from raw evidence and select/reject a production candidate; **recency alone cannot win**"* |
| 5 | **Layout V2 onayı** | `.codex/CURRENT.md:169-172` | Ownership, kompozisyon, responsive-model, temel kolon/detay veya kasıtlı baseline değişiklikleri **açık Layout V2 onayı** ister; V1.x yalnız V1 yasalarını koruyan performans/doğruluk işidir |
| 6 | Change Pipeline T3.1 devam zamanı | `.codex/MEMORY.md` | Ürün fazı kapanınca |

### R8 — Test kanıtının bağımsız doğrulanmamışlığı

Süreklilik dosyaları (`.codex/CURRENT.md`, `.codex/MEMORY.md`, `.planning/STATE.md`, `.codex/NEXT-SESSION-PROMPT.md`) şu gate'leri raporluyor: full nextest **3.683/3.683 + 6 intentional skip**, Chromium **35/35**, Python **68/68**, Bun **5/5 + 12/12**, fmt + Linux all-target Clippy + Windows MSVC Clippy temiz, graph **24.357/129.888**.

**Bu dört kaynak bağımsız DEĞİL** — hepsi aynı ajan zincirinin yazdığı korelasyonlu belgeler. evidence-propagation θ kuralı: korelasyonlu kaynaklar tek kaynak sayılır; `verified` için 1 executable/official ≥0.9 VEYA 2 BAĞIMSIZ kaynak ≥0.7 gerekir.

Bu analiz salt-okuma kapsamında yapıldığı için testler çalıştırılmadı. **Confidence 0.6 — belge kanıtı, executable kanıt değil.** Yayın/karar öncesi bağımsız `just check` (veya justfile alt-reçeteleri; `just` bu makinede eksik olabilir — `.codex/MEMORY.md` operasyonel dersi) çalıştırılması önerilir.

---

## G. SIRADAKİ MANTIKLI ADIMLAR

Vizyondan geriye doğru türetilmiş, önceliklendirilmiş:

| # | Adım | Neden (vizyona bağ) | Önkoşul | Öncelik |
|---|---|---|---|---|
| **1** | **DCLICK-6 + FFO-9 fiziksel E2E'yi tamamla** (`HERDR_RENDER_PROF=1 ./.local/herdr-trail-test.sh run`) | İki atom "otomatik kapalı, fiziksel açık". Yeni faz açmadan önce mevcut atomu kapat — sürekli entegrasyon ilkesi (big-bang değil) | Yok, hemen | **P0** |
| **2** | **Bağımsız gate doğrulaması** — `just check` veya justfile alt-reçeteleri tek seferlik çalıştır | R8: tüm test iddiaları korelasyonlu belge kanıtı. Karar öncesi executable çapa gerekli (core-principles §2) | 1 | **P0** |
| **3** | **Custom Layout B1: cartography** — `.cartography/custom-layout-SYSTEM-MAP.json` üret; 7 mockup bölgesini mevcut seam'lere eşle | Fork vizyonunun ikinci ekseni; **belge render'ın hangi bölgede yaşayacağını da bu belirler**. Tüm SF/FM önkoşulları kapalı | 1 | **P1** |
| **4** | **Custom Layout B2: design spec** — `docs/superpowers/specs/…custom-layout-design.md`; bölge sözleşmeleri + runtime/client sınıflandırması + no-goals | PRD'nin şart koştuğu kapı (`superpowers:brainstorming` → `writing-plans`). Ayrıca S5-tuzağından (arbitrary registry) korunma noktası | 3 | **P1** |
| **5** | **Belge önizleme kararı** — FMR-5 P5 plugin adapter mı, native render mi? Açık PRD ile karara bağla | Mevcut FMR-5 hybrid sınırı native ağır render'ı DIŞLIYOR. Kullanıcı talebi bu sınırı test ediyor → sınır ya korunur ya **açıkça revize edilir** | 4 (bölge kararı) | **P1** |
| **6** | **Fork sürüm/dağıtım stratejisi kararı** | R2: 819 commit kullanıcıya ulaşmıyor. Seçenekler: (a) fork'a özgü sürüm + kendi install akışı, (b) izole build'i kalıcı kullanım binary'sine terfi, (c) upstream'e Discussion açarak file-manager yönünü tartışmak | Yok | **P1** |
| **7** | **Upstream 4 commit'ini absorbe et** (`git merge-tree` salt-okuma testi önce, sonra `git merge upstream/master`) | R3: drift maliyeti zamanla artar; küçükken al | 2 | P2 |
| **8** | **Takipsiz bilgi katmanını koru** — `docs/patterns/`, `docs/references/`, `docs/analysis/`, `custom-layout-architecture-guide.md`, `.cartography/` için `.gitignore` istisnası veya harici arşiv | R4: kurumsal hafıza git korumasında değil; `reference-registry` kuralının gereği | Yok | P2 |
| **9** | **`custom-layout-architecture-guide.md` durum tablosunu güncelle** (SF5/SF6/FM1-5 → CLOSED) | R5: yanlış durum bilgisi gelecek ajanı yanıltır | Yok | P2 |
| **10** | **Kalan P0 kuyruğunu kapat:** FIP-6.3/6.7/6.8, FMR-0 sıralama kararı, FMR-4 lisans re-doğrulama | %95,1 tamamlanmış ürün kuyruğunda kalan 24 madde; B4 yürütmesinden önce temizlenmeli | 1-2 | P2 |
| **11** | **Change Pipeline T3.1 devam kararı** | Ürün fazı kapanınca çözülecek bilinçli duraklatma | 10 | P3 |
| **12** | **`src/app/input/file_manager.rs` (10.833 satır) bölünme değerlendirmesi** | R6: custom layout işi bu dosyaya dokunacak; AGENTS.md refactor-risk protokolü karakterizasyon testi ister | 4 | P3 |

---

## H. KARAR GEÇMİŞİ ARŞİVİ

Bu projede alınmış ve kayıtlı olan **tüm yön kararları**. Gelecekte *"bu neden böyle?"* sorusunun tek adresi bu tablodur.

**Durum sözlüğü:** `YÜRÜRLÜKTE` = bugün hâlâ geçerli · `SÜPERSEDED` = sonraki kararla değiştirildi ama tarihsel bağlamı korunuyor · `ERTELENDİ` = tetik bekliyor · `İPTAL` = geri alındı

### H.1 Ürün yönü kararları

| # | Tarih | Karar | Gerekçe (alıntı) | Kaynak | Durum |
|---|---|---|---|---|---|
| K1 | 2026-07-13 | **Native FM inşa et, yazi'yi gömme** | *"yazi'ye İHTİYACIMIZ YOK. Dosya yöneticisini herdr'a NATIVE (Rust/ratatui, Lua'sız) inşa etmek hem daha temiz hem hedefe daha uygun."* Üç bağımsız kanıt hattı: (1) yazi'nin TUI çekirdeği zaten ratatui = bizde, Lua scaffolding atılır; (2) Files tab greenfield (`src/ui/sidebar.rs:930` "placeholder") → duplicate riski yok; (3) image preview native'de daha iyi | `.local/prd/native-file-manager-DECISION.md:13` | **YÜRÜRLÜKTE** |
| K2 | 2026-07-13 | **Image preview: Path β (native local placement)**, Path α (yazi-in-pane passthrough) reddedildi | Path α TERM-probe kırılganlığı taşıyor: herdr child pane'e sabit `TERM=xterm-256color` veriyor → yazi hiçbir brand string'iyle eşleşmez → 1000ms blocking canlı probe'a düşer. Path β yazi'nin `Image::downscale` saf image-processing'ini alıp herdr'ın kendi `KittyImagePlacement` motoruna besler; `encode_graphics_update` **kaynak-agnostik** (satır 267), dedup/diff/tmux-frame 12 test HAZIR | `native-file-manager-DECISION.md` §2 | **YÜRÜRLÜKTE** (B0/B2 kapalı) |
| K3 | 2026-07-15 | **12 ordered phase onayı: SF0-SF6 → FM1-FM5** | *"The user has now supplied independent concrete product demand that was absent at the P4.0 checkpoint: Files must become a real app surface instead of a terminal curtain; AppDock/WorkspaceStage must exist..."* | `.codex/TASKS.md` "Active Product Program" | **YÜRÜRLÜKTE** (hepsi kapalı) |
| K4 | 2026-07-17 | **Files Interaction Polish (FIP) programı onayı** | Kullanıcı açık onayı 2026-07-17 | `docs/superpowers/specs/2026-07-17-herdr-files-interaction-polish-design.md` | **YÜRÜRLÜKTE** (%87,5) |
| K5 | 2026-07-17 | **Drag-and-drop MVP'den ve programdan ÇIKARILDI** | *"Drag-and-drop status: **explicitly removed from the MVP and from this program**"*; ayrıca no-goals: *"drag-and-drop from Files to chats, Spaces, Projects, or Agents"* | `docs/superpowers/specs/2026-07-17-herdr-files-interaction-polish-design.md:10,44,89,883` · `.codex/MEMORY.md:90` · `.codex/TASKS.md:197` | **YÜRÜRLÜKTE** |
| K6 | 2026-07-18 | **Custom Layout kanonik öncelik direktifi** | *"after FIP-1 and FIP-2 close, build the custom-layout infrastructure so that composing this layout is EASY, FAST, and production-grade. **Priority #1 of the whole effort: a file manager better than yazi and superfile**"* | `.local/prd/custom-layout-target-mockup.md:3-6` | **YÜRÜRLÜKTE** (B1-B4 başlamadı) |
| K7 | 2026-07-19 | **Files Layout V1 KİLİDİ** | Kullanıcı mevcut Native Files kompozisyonunu `Files Layout V1` olarak onayladı. Freeze checkpoint `d98c31c70946e496cb6536f02fc96e45974df2de`. V1 yasaları: global agent/workspace tracker · Files-local Favorites/Locations rail veya compact drawer · exact `Location(path)`/`Direct(path)` origin · scrollable Miller Trail · detail · mixed mtime grupları · 1/3 yatay scroll · disjoint current-frame geometri · bounded FM I/O lane. **V1.x** = bu yasaları koruyan performans/doğruluk işi; **V2** = ownership/kompozisyon/responsive/temel kolon değişikliği → açık onay ister | `docs/superpowers/specs/2026-07-19-herdr-files-layout-v1-lock.md` · `.codex/CURRENT.md:161-172` | **YÜRÜRLÜKTE** |
| K8 | 2026-07-19 | **FCL Option A: Locations rail Files-local olur** | Kullanıcı Option A'yı onayladı: global sol panel agent/workspace runtime tracking'i korur; Favorites/Locations tam-yükseklik Files-local rail'e taşınır; Miller Trail sağında yatay scroll'da kalır | `.codex/CURRENT.md:195-205` | **YÜRÜRLÜKTE** |
| K9 | 2026-07-21/22 | **FMN + stutter fix fiziksel KABUL** | Kullanıcı izole canlı denemeyi tamamladı, *"original freezing/stutter appears completely gone and interaction works very well"*; FMN build'i *"works perfectly"* | `.codex/CURRENT.md:109-111` · `.codex/MEMORY.md` | **YÜRÜRLÜKTE** |
| K10 | 2026-07-23 | **DCLICK: primary click = odak, aktivasyon DEĞİL** | Kullanıcının fiziksel raporu eski `click = directory activation` binding'ini geçersiz kıldı. Kök neden: `handle_file_manager_row_mouse` → `queue_file_manager_trail_directory_activation` → `TrailActivateOutcome::Branched` `active_col`'u çocuğa taşıyordu | `.codex/TASKS.md` P0 ACTIVE DCLICK · `.codex/evidence/files-directory-click-focus-closure.md` | **YÜRÜRLÜKTE** |

### H.2 Mimari NO-GO kararları (kanıtla reddedilenler)

| # | Tarih | Karar | Gerekçe (alıntı) | Kaynak | Durum |
|---|---|---|---|---|---|
| K11 | P4.0 | **S5 ComponentRegistry: NO-GO** | Mevcut kanıt: `Compositor` iki sabit `Component` katmanı; `BaseLayer` tek açık içerik takası; dinamik kayıt/per-component event ownership/ikinci sayfa lifecycle YOK. Eksik tetik: *"A second real independently owned component/page that duplicates render, hit-area, lifecycle, and event routing"*. Karar: *"Implementation NO-GO; keep the concrete content-swap pattern"* | `.codex/TASKS.md` P4.0 matrisi | **YÜRÜRLÜKTE** (kalıcı) |
| K12 | P4.0 | **S7 Popup ownership stack: NO-GO** | Tek `Mode` tek `OverlayLayer` seçiyor; `render_modal_shell` 8 çağıran, `modal_stack_areas` 10; context/modal transition testleri focus/close order'ı zaten koruyor. Eksik tetik: gerçek eşzamanlı iç içe popup | `.codex/TASKS.md` P4.0 matrisi | **YÜRÜRLÜKTE** (kalıcı) |
| K13 | P4.0 | **S6 Resizable persisted shell: NO-GO → sonra SÜPERSEDED** | İlk karar: *"Implementation NO-GO; preserve current snapshot compatibility"*. Sonra: *"S6 activation gate superseded by later explicit product demand and absorbed into active SF0-SF6"* | `.codex/TASKS.md` | **SÜPERSEDED** (SF0-SF6'ya absorbe) |
| K14 | N2.0 | **N2 Dynamic/unbounded Miller: NO-GO; yalnız N2.1 GO** | Pinned Yazi `4dab4803` + Joshuto `d2581fb0` + Ranger/Yazi primary docs incelemesi: *"All inspected products use a bounded parent/current/preview projection and Herdr already provides responsive 1/2/3-column projection plus cached context refresh."* Yalnızca "path-stable parent return" gerçek delta → N2.1 GO. Bütçe: sıfır yeni state field/history, ekstra dizin okuması yok | `.codex/evidence/n2-path-stable-miller-navigation.md` · `.codex/TASKS.md` N2.0 Final Decision | **YÜRÜRLÜKTE** |
| K15 | N2.2 | **N2.2 gate SÜPERSEDED** | *"superseded by later explicit horizontal/Finder-like demand and absorbed into FM1-FM4 with finite 32-segment/5-resident bounds"* | `.codex/TASKS.md` | **SÜPERSEDED** (FM1-FM4'e absorbe) |
| K16 | M3.0 | **M3 Genel UI panel/sayfa/buton arayüzü: NO-GO** | Evidence-backed implementation NO-GO | `.codex/evidence/m3-general-ui-interface.md` · `.codex/TASKS.md` M3.0 Final Decision | **YÜRÜRLÜKTE** |
| K17 | FM5 | **RightPanel + adaptive hybrid preview: NO-GO — inline preview KALIR** | *"**NO-GO for Shell `RightPanel` and adaptive hybrid product work. Keep the existing inline final Miller preview column.**"* B prototipi 32-hücre RightPanel ekleyip inline preview'ı kaldırıyordu; C hybrid ikisini birden gerektiriyordu. B/C yeni RightPanel focus/scroll owner + deterministik transfer gerektirir; bütçe içinde kalmıyor. *"This is not a claim that a RightPanel can never be useful"* — gelecekte gerekirse explicit RightPanel component/focus/hit ownership tasarımı şart | `.codex/evidence/fm5-preview-placement-decision.md:11-12,183-194` | **YÜRÜRLÜKTE** |
| K18 | FMR-3 | **PDF/office/arşiv/medya: metadata-only** | Native capability matrisi: pure native text/image · metadata-only · optional-plugin · unsupported. PDF, office, archive, audio, video, binary, broken/special, oversized, control, non-UTF-8 → metadata-only veya optional-plugin | `.codex/evidence/files-preview-capability-test-points.md` · `src/fm/preview_capability.rs:129` | **YÜRÜRLÜKTE** |
| K19 | FMR-5 | **Hybrid sınır: native = hafif bounded preview, plugin = ağır expert pane** | *"Select hybrid boundary: native core owns directory/path/Trail/mouse truth and lightweight bounded preview; **optional plugins own heavyweight expert panes**"*; plugin adapter *"opens a plugin pane and **never injects renderer output into native Ratatui cells**"* | `.codex/TASKS.md` FMR-5 · `docs/superpowers/plans/2026-07-18-herdr-files-visibility-preview-plugin-integration.md` Task 5 | **YÜRÜRLÜKTE** — belge render talebi bu sınırı test ediyor |
| K20 | FMN-0 | **Yazi'nin unbounded History cache'i REDDEDİLDİ** | Yazi kaynak commit `6d84921e7004eb8d49ba13a4acc97c6cfeb094b4`: cursor/activation ayrımı · discardable async folder preview · ticketed stale-result rejection · change-gated rendering · **unbounded directory history**. *"Transfer the first four laws; **reject the unbounded cache**"* | `.codex/references/yazi-file-manager-performance-transfer.md` · `.codex/MEMORY.md` | **YÜRÜRLÜKTE** |
| K21 | FMN-6 | **Dizin pre-warm: ölçüm-first, yetkisiz** | *"Home/Desktop/Downloads pre-warm is **not authorized without a separate first-entry latency RED**. Any future implementation is allowlisted, background, mtime-invalidated, and capped per directory by entries and bytes; **no general LRU**"* | `.codex/MEMORY.md` · `.codex/TASKS.md` FMN-6 | **ERTELENDİ** (tetik: reproducible first-entry RED) |
| K22 | FIP | **No-submit invariant MUTLAK** | *"the selected safe UTF-8 file or directory path is inserted once into an explicitly selected live agent terminal with **no CR/LF/Enter, submit, implicit whitespace, or implicit split/chat**. All stale identity, path-kind, control-character, and backpressure cases fail closed"* | `.codex/MEMORY.md` | **YÜRÜRLÜKTE** |

### H.3 Süreç ve disiplin kararları

| # | Tarih | Karar | Gerekçe (alıntı) | Kaynak | Durum |
|---|---|---|---|---|---|
| K23 | — | **Upstream'e push YASAK** | *"The acting GitHub account is `CyPack`; this is external-contributor/fork work. **Never push upstream or open upstream issues/PRs for the user.**"* + *"**No push to `upstream`, no force push**, and no unrelated staging"* | `.codex/MEMORY.md` · `.codex/CHANGE-PIPELINE-TASKS.md` Global Constraints | **YÜRÜRLÜKTE** |
| K24 | — | **Standing authorization: otonom targeted commit + CyPack-only FF push** | *"The user granted standing authorization for autonomous targeted commits and CyPack fork-only fast-forward pushes. Preserve all verification and atomicity gates, but **do not repeatedly ask for commit-message alignment**"* | `.codex/MEMORY.md` · `.codex/BOOTSTRAP.md:27-29` | **YÜRÜRLÜKTE** |
| K25 | — | **Stable Herdr / socket / config'e ASLA dokunma** | *"Stable installed Herdr and development Herdr must remain isolated. Use `.local/ISOLATED-DEV-TEST.md` for runtime checks."* + *"Stable Herdr processes and inherited stable sockets are never touched"* | `.codex/MEMORY.md` · `.codex/CHANGE-PIPELINE-TASKS.md` | **YÜRÜRLÜKTE** |
| K26 | T0 | **Change Pipeline T3.1'de DURAKLATILDI** | *"The non-product change-pipeline lane is paused at T3.1 until the current sequential product phase closes. **Product and tooling commits never mix.**"* Ayrıca: *"Only one macro task and one micro task are active at a time unless an approved plan explicitly proves safe parallel ownership"* | `.codex/MEMORY.md` · `.codex/CHANGE-PIPELINE-TASKS.md` | **ERTELENDİ** (tetik: ürün fazı kapanışı) |
| K27 | — | **Görsel kabul MUTLAKA Playwright Chromium** | *"FIP visual acceptance requires Playwright Chromium driven by deterministic Ratatui cell fixtures, while **Rust and isolated PTY tests retain semantic and byte-level authority**"*; *"`--update-snapshots` yalnız YENİ baseline için, mutation kanıtı ham buffer karşılaştırması"* | `.codex/MEMORY.md` · `fip-closure-and-custom-layout-prd.md:87-89` | **YÜRÜRLÜKTE** |
| K28 | — | **Kanıt-önce-iddia (evidence before claims)** | *"No completion, graph-freshness, or publication claim without fresh evidence"*; *"`index_status=ready` can still be stale; verify a recent symbol"*; *"A human 'stutter is gone' report is **qualitative symptom acceptance**. Keep it distinct from profiler counts, structural tests, and fresh publication gates"* | `.codex/CHANGE-PIPELINE-TASKS.md` · `.codex/MEMORY.md` | **YÜRÜRLÜKTE** |
| K29 | — | **RED-önce-production (TDD)** | *"**No production code without an observed behavior-specific RED test**"* | `.codex/CHANGE-PIPELINE-TASKS.md` Global Constraints | **YÜRÜRLÜKTE** |
| K30 | — | **`git add -A` YASAK** | *"Never bulk-stage with `git add -A`; local cartography and continuity artifacts may be present"* | `.codex/MEMORY.md` Known Operational Lessons | **YÜRÜRLÜKTE** |
| K31 | — | **Referans benzerliği kanıttır, uygulama yetkisi DEĞİL** | *"Reference similarity is evidence, not implementation authority"* | `.codex/CHANGE-PIPELINE-TASKS.md` | **YÜRÜRLÜKTE** |
| K32 | 2026-07-15 | **Completion audit: 13 core modül + N1/N3/N4/N2.1/M1/M2 kapalı; 4 trigger-gated future item korundu** | *"At the completion-audit checkpoint the product queue intentionally contained only four trigger-gated future items; **that absence was a verified architecture decision, not missing decomposition**"* | `.codex/evidence/native-fm-completion-audit.md` · `.codex/TASKS.md` P0 Completion Audit | **YÜRÜRLÜKTE** (S5/S7 hâlâ gated) |

### H.4 Upstream'in kendi kararları (fork'u bağlayan)

| # | Karar | Gerekçe (alıntı) | Kaynak | Durum |
|---|---|---|---|---|
| K33 | **Issue tracker = maintainer iş kuyruğu** | *"Issues are only for reproducible bug reports and maintainer-created or maintainer-converted work items."* Feature request/fikir/soru/katkı önerisi → **Discussion** | `CONTRIBUTING.md` | YÜRÜRLÜKTE |
| K34 | **İlk katkıda approval gate** | *"Before opening your first PR, get maintainer approval on an accepted issue... A maintainer will comment `/approve @your-github-username`"*; gerekçe: *"AI makes it trivial to generate plausible-looking contributions that do not fit the app"* | `CONTRIBUTING.md` · `.github/workflows/approve-contributor.yml` | YÜRÜRLÜKTE |
| K35 | **AI ajanları kullanıcı adına issue AÇAMAZ** | *"Do not use the GitHub CLI, API, browser automation, or any other tool to submit an issue on their behalf"* | `CONTRIBUTING.md` · `AGENTS.md` | YÜRÜRLÜKTE |
| K36 | **Runtime/client boundary guardrail** | *"New work should not deepen the current server/TUI coupling... Do not add new shared behavior that only works through the private TUI client socket"* | `AGENTS.md` | YÜRÜRLÜKTE (fork uyuyor) |
| K37 | **Stable docs ≠ unreleased docs** | Root `README.md`/`CHANGELOG.md`/`website/src/content/docs/` = yayınlanmış sürüm; unreleased → `docs/next/` | `AGENTS.md` · `CONTRIBUTING.md` | YÜRÜRLÜKTE |
| K38 | **Closing keyword YASAK; `refs #N` kullanılır** | *"Herdr closes released issues after a release is published, not when unreleased commits land on `master`"* | `AGENTS.md` · `CONTRIBUTING.md` | YÜRÜRLÜKTE |
| K39 | **Detection: manifest hot-reload + kanıt döngüsü** | Ekran-okuma kırılganlığı → binary güncellemesi gerektirmeyen manifest'ler; *"do not use the user-visible viewport for agent status because users can scroll it"* | `AGENTS.md` · blog satır 122-137 | YÜRÜRLÜKTE |
| K40 | **Preview kanalı opt-in, Homebrew/Nix stable-only** | Stable ve preview aynı `master`'dan; uzun ömürlü preview branch YOK | `AGENTS.md` "Release Channels" | YÜRÜRLÜKTE |

---

## I. VİZYON SORULARI YENİDEN AÇILIRSA OKUNACAK KAYNAKLAR

Bu bölüm, ileride *"yeni ihtiyaçlarımız olduğunda / daha fazlasını istediğimizde"* benzer bir değerlendirme turuna girildiğinde **giriş kapısıdır**.

### I.1 Okuma sırası (bu turda kanıtlanmış sıra)

| Sıra | Kaynak | Ne verir |
|---|---|---|
| 1 | `README.md` · `Cargo.toml` · `SKILL.md` · `SPONSORS.md` | Kimlik cümleleri (V1-V5), lisans, iş modeli |
| 2 | `website/src/content/blog/coding-agents-are-becoming-runtimes.md` | **Vizyon manifestosu** — en derin niyet beyanı |
| 3 | `website/src/pages/compare.astro` | Rakip konumlandırma + "hangi kesişimdeyiz" tezi |
| 4 | `website/src/content/docs/concepts.mdx` · `index.mdx` | Ürün kavram sözlüğü + hedef kitle giriş yolları |
| 5 | `AGENTS.md` (= `CLAUDE.md`) | **Mimari anayasa** + runtime/client guardrail + release/commit disiplini |
| 6 | `CONTRIBUTING.md` | Katkı/fork disiplini, external contributor guardrail |
| 7 | `.codex/CURRENT.md` (başlık taraması önce: `grep -n '^#\{1,3\} '`) | Aktif override'lar, verified checkpoint zinciri |
| 8 | `.codex/TASKS.md` (başlık + `[x]/[ ]` sayımı) | Faz yapısı ve tamamlanma |
| 9 | `.codex/MEMORY.md` | **Karar defteri** (stable facts + decision ledger + operational lessons) |
| 10 | `.codex/CHANGE-PIPELINE-TASKS.md` | Araç lane durumu + global constraints |
| 11 | `.local/prd/*.md` | Fork'a özgü ürün niyeti (native FM kararı, custom layout mockup) |
| 12 | `docs/superpowers/specs/` + `plans/` | Dondurulmuş tasarım/plan sözleşmeleri |
| 13 | `.codex/evidence/*.md` (41 dosya) | Faz kapanış kanıtları |
| 14 | `.planning/STATE.md` · `.codex/NEXT-SESSION-PROMPT.md` | Son oturum durumu + kanonik tetik |

### I.2 Divergence ölçüm komutları (kopyala-çalıştır)

```bash
cd /home/ayaz/projects/herdr

# Fork ↔ upstream mesafesi — HER İKİ YÖNÜ de ölç, tek yön yanıltır
git rev-list --count origin/master..upstream/master   # upstream'de olup fork'ta OLMAYAN
git rev-list --count upstream/master..origin/master   # fork'ta olup upstream'de OLMAYAN
git merge-base origin/master upstream/master
git log -1 --format='%ci %h %s' $(git merge-base origin/master upstream/master)

# Aktif branch ↔ fork master
git rev-list --count origin/master..feat/native-fm
git rev-list --count feat/native-fm..origin/master

# Fork commit'lerinin profili
git log --format='%an' upstream/master..origin/master | sort | uniq -c | sort -rn
git log --format='%s' upstream/master..origin/master | grep -oE '^[a-z]+' | sort | uniq -c | sort -rn
git log --format='%ci' upstream/master..origin/master | tail -1   # en eski
git log --format='%ci' upstream/master..origin/master | head -1   # en yeni

# Upstream'de olup alınmayanlar (isim isim)
git log --oneline origin/master..upstream/master
```

### I.3 Metrik hesaplama komutları

```bash
# Kod ölçeği
find src -name '*.rs' | xargs cat | wc -l                    # toplam Rust LOC
find src -name '*.rs' | xargs wc -l | sort -rn | head -22    # en büyük dosyalar
grep -rn "#\[cfg(test)\]" --include="*.rs" src/ | wc -l      # test bloğu sayısı

# Faz tamamlanma (deterministik)
grep -oE '^\s*-?\s*\[[ x]\]' .codex/TASKS.md | sort | uniq -c
grep -oE '^\s*-?\s*\[[ x]\]' .codex/CHANGE-PIPELINE-TASKS.md | sort | uniq -c

# Mimari ilke doğrulaması
grep -rn "cfg(target_os" --include="*.rs" src/ | grep -v "^src/platform/" | wc -l
grep -n "fn compute_view\|pub fn render(" src/ui.rs
grep -rn "pub struct PaneState\|pub struct PaneRuntime" src/

# Doküman takip durumu
comm -23 <(find docs -type f | sort) <(git ls-files docs/ | sort)

# Graph (codebase-memory-mcp)
#   index_status(project="home-ayaz-projects-herdr")
#   → ready DEĞİL, TAZE SEMBOL ile doğrula (örn TrailSnapshots::focus_entry)
```

### I.4 TUZAKLAR (gerçek vakalarla)

| # | Tuzak | Gerçek vaka | Korunma |
|---|---|---|---|
| **T1** | **Divergence yönünü ters okumak** | 2026-07-24 turunda görev tanımında *"master 742 commit geride"* yazıyordu. Gerçek ölçüm: fork **777 commit ÖNDE**, 4 geride. Ters okuma tüm risk analizini tersine çevirirdi ("upstream'i yakala" vs "fork'u yayınla" — tamamen farklı stratejiler) | `rev-list --count` **her iki yönde** çalıştır, sonucu cümleyle yaz: "X'te olup Y'de olmayan = N" |
| **T2** | **`find \| head -N` kesmesi anomali sanmak** | Aynı turda `find docs -type f \| head -80` çıktısı kesildi → CURRENT.md'nin atıf yaptığı 5 spec dosyası "eksik" sanıldı → sahte anomali. Tam listeleme hepsinin var olduğunu gösterdi | Dosya varlığı iddiası için `head` KULLANMA; `ls -la <dizin>` veya `test -f` döngüsü |
| **T3** | **`index_status: ready`'yi tazelik sanmak** | `.codex/MEMORY.md` operasyonel dersi: *"`index_status=ready` can still be stale; verify a recent symbol."* Ayrıca: built-in MCP kanalı CLI store'dan **eski** olabilir (24.217 vs 24.357 gözlendi) | `search_graph` ile son eklenen sembolü ara (örn `TrailSnapshots::focus_entry`); CLI kanıtını built-in kanıtı diye etiketleme |
| **T4** | **Korelasyonlu belgeleri bağımsız kanıt saymak** | CURRENT.md + MEMORY.md + STATE.md + NEXT-SESSION-PROMPT.md aynı test sayılarını raporluyor. Dördü de aynı ajan zincirinin çıktısı → **tek kaynak** | θ kuralı: `verified` = 1 executable/official ≥0.9 VEYA 2 BAĞIMSIZ ≥0.7. Test iddiaları için komutu çalıştır veya conf 0.6 işaretle |
| **T5** | **`unwrap()` ham sayısını ihlal sanmak** | 3.344 `.unwrap()` görünüyor ama 461 `#[cfg(test)]` bloğu production dosyalarının İÇİNDE. Kaba grep ayıramıyor | Projenin kendi yöntemini kullan: diff-tabanlı "added-production-unwrap" taraması |
| **T6** | **Bayat durum tablolarına güvenmek** | `custom-layout-architecture-guide.md` hâlâ "SF5.1 NEXT, FM1-5 pending" diyor; gerçekte hepsi kapalı (7 gün bayat) | Durum için **`.codex/TASKS.md` `[x]` sayımını** otorite say; rehber/guide belgeleri ikincil |
| **T7** | **zsh glob tuzağı** | `grep ... --include=*.rs` zsh'de `no matches found` hatası verir (tırnaksız glob) | `--include="*.rs"` tırnak içinde yaz |
| **T8** | **`just` var sanmak** | `.codex/MEMORY.md`: *"`just` may be absent. Read `justfile` and run the entire recipe directly rather than claiming a partial gate"* | `just` yoksa justfile'ı oku, alt komutları tek tek çalıştır, "wrapper geçti" DEME |

### I.5 Bu turda kullanılan metodoloji özeti

1. **Cartographer refleksi:** repo topografyası → git durumu → doküman ağacı envanteri → büyük dosyalarda başlık taraması (`grep -n '^#'`) → hedefli bölüm okuma
2. **Graph-before-grep:** `index_status` → `get_architecture` → `search_graph` (name_pattern) → grep ile çapraz doğrulama
3. **Evidence-propagation:** her iddia (claim, evidence, confidence); korelasyonlu kaynaklar tek sayıldı; negatif grep'ler confidence düşürülerek kaydedildi
4. **Loop variant (V):** açık düğüm = {vizyon kaynağı okunmamış, yetenek kanıtsız, faz durumu belirsiz}; her turda azaldı; V=0'da durduruldu
5. **Salt-okuma disiplini:** kod/git mutasyonu yapılmadı; `.superpowers/` ve stable socket'e dokunulmadı

---

## J. BU TURDA İNCELENMEYEN VİZYON EKSENLERİ

Bu turun scope'u ürün vizyonu + misyon durumu + faz haritası + fork/upstream ilişkisiydi. Aşağıdaki eksenler **bilinçli olarak kapsam dışı** bırakıldı. Her biri için: neden dışarıda kaldı, hangi soru için bakılmalı, giriş kaynağı.

| # | Eksen | Neden kapsam dışıydı | Hangi soru için bakılmalı | Giriş kaynağı |
|---|---|---|---|---|
| **J1** | **Plugin marketplace ekonomisi** | Ürün vizyonu turu; marketplace ticari/ekosistem boyutu ayrı analiz | "Belge önizleme plugin'i yazarsak dağıtımı nasıl olur? Marketplace denetimsiz — güven modeli ne? Plugin'ler para kazanabilir mi?" | `workers/plugin-marketplace/src/index.ts` · `website/src/pages/plugins.astro` · `docs/marketplace.mdx` (*"automatic index... not a reviewed catalog"*) · `docs/plugins.mdx#trust-and-security` |
| **J2** | **Sponsorluk + ticari lisans modeli** | Vizyonun finansmanı; ürün kararlarını doğrudan bağlamıyor | "Fork'un kendi dağıtımı olursa AGPL/ticari lisans nasıl etkilenir? Upstream'in sponsorluk geliri fork'tan etkilenir mi?" | `SPONSORS.md` · `LICENSE` (34.880 byte) · `README.md:83-88` · `.github/FUNDING.yml` |
| **J3** | **i18n / çeviri stratejisi** | Fork'un iş ekseninde değil | "Native FM yayınlanırsa ja/zh-cn çevirisi gerekir mi? Çeviri parity nasıl korunuyor?" | `website/src/content/docs/{ja,zh-cn}/` (17'şer sayfa) · `scripts/docs_translation_parity.py` + testi · `.github/ISSUE_TEMPLATE/translation.yml` |
| **J4** | **Windows beta yol haritası** | Fork Linux'ta geliştiriyor; Windows validation ayrı VM akışı | "Native FM Windows'ta çalışır mı? ConPTY/clipboard/path semantiği FM'i nasıl etkiler? VM validation ne zaman gerekli?" | `src/platform/windows.rs` · `docs/windows-beta.mdx` · `scripts/windows_smoke_conpty_path.ps1` · `AGENTS.md` "Windows VM validation" (yalnız Can'ın makinesinde) · `install.ps1` |
| **J5** | **Mobile / dar ekran ürün konumu** | FM'in dar ekran davranışı Layout V1'de "compact drawer" olarak çözülmüş; genel mobile stratejisi ayrı | "Custom layout 7 bölgesi dar ekranda nasıl davranır? Mobile switcher ile Stage nasıl ilişkilenir?" | `src/ui/mobile.rs` · issue `#316` mobile-width-threshold · `docs/next/CHANGELOG.md` "mobile switcher now starts from an agents-first summary" |
| **J6** | **Agent detection ekosistem genişlemesi** | Upstream'in ana ekseni; fork'un FM işine değmiyor | "Yeni agent eklemek fork'u nasıl etkiler? Blog'un vaat ettiği **per-agent support matrix** ne zaman yayınlanacak (grid satır 12)? Manifest hot-reload dağıtımı (`website/agent-detection/`) nasıl çalışıyor?" | `src/detect/manifests/` (19) · `website/agent-detection/` (20) · `src/detect/manifest_update.rs` · `AGENTS.md` "Agent Detection Updates" · blog satır 139-151 |
| **J7** | **Multi-monitor / paylaşımlı görünüm** | Araştırma notu var, ürün kararı yok | "Custom layout multi-monitor'ü kapsamalı mı? Aynı session'ı iki ekranda farklı bölgelerle görmek mümkün mü?" | `research/multi-monitor-shared-view.md` (7.589 byte — **bu turda okunmadı**) |
| **J8** | **Socket API / protokol evrimi** | Fork protokole dokunmuyor (FFO diff `src/protocol` boş) | "Belge önizleme veya custom layout server-side state gerektirirse protokol nasıl büyür? `PROTOCOL_VERSION` bump kuralı ne?" | `src/protocol/wire.rs` · `docs/next/api/herdr-api.schema.json` · `AGENTS.md` "When changing the server/client wire protocol..." |
| **J9** | **Vendored libghostty-vt bakım yükü** | Fork vendor'a dokunmadı | "Upstream libghostty-vt güncellerse fork nasıl etkilenir? Aktif local patch'ler neler?" | `vendor/libghostty-vt.vendor.json` · `vendor/libghostty-vt.patches.md` · `vendor/patches/libghostty-vt/` · `scripts/vendor_libghostty_vt.py` |
| **J10** | **Performans bütçeleri ve SSH profili** | FMP kapandı ama sistematik bütçe rejimi ayrı analiz | "Custom layout 7 bölge SSH'de kaç hücre/frame maliyeti getirir? SF6.3/FM2.2 bütçe gate'leri neyi ölçüyor?" | `docs/superpowers/specs/2026-07-17-herdr-custom-layout-architecture-guide.md` §4 · `src/render_prof.rs` · `.codex/evidence/files-rapid-navigation-scale-calibration.md` · `.local/perf/` |
| **J11** | **Test mimarisi ve flake yönetimi** | Gate sonuçları okundu ama test mimarisi analiz edilmedi | "3.683 test nasıl organize? Bilinen flake sınıfları neler? Görsel oracle determinizmi nasıl sağlanıyor?" | `tests/` (12 entegrasyon dosyası + `support/`) · `tests/visual/` · `.codex/MEMORY.md` flake dersleri · `src/ui/visual_fixture.rs` |
| **J12** | **Codex/Claude çift-ajan süreklilik sistemi** | Meta-katman; ürün vizyonu değil | "İki CLI arasında devir nasıl çalışıyor? `.codex/` yapısı neden bu şekilde?" | `.codex/README.md` · `.codex/MEMORY-SYSTEM.md` · `.codex/BOOTSTRAP.md` · `.codex/skills/` (4 skill) · `.local/AGENTIC-DEV.md` · **related doc:** `docs/analysis/2026-07-24-chat-forensics-codex-cursor-handover.md` |

---

## K. KANIT SÖZLEŞMESİ VE ÖLÇÜM NOTLARI

### K.1 Kanıt kategorileri ve güven düzeyleri

| Kategori | Kaynak tipi | Güven | Not |
|---|---|---|---|
| Git ölçümleri (819 commit, tarih, yazar, tip dağılımı) | executable (komut çıktısı) | **0.95** | `git rev-list --count`, `git log --format` |
| Dosya/LOC sayımları | executable | **0.95** | `find`, `wc -l`, `ls` |
| Kod yapısı (`StageSurfaceView`, `compute_view`/`render` imzaları, `src/ui/shell/`, `src/fm/`) | source + graph — **iki bağımsız yöntem** | **0.9-0.95** | codebase-memory `search_graph` + grep çapraz doğrulaması |
| Vizyon beyanları (README, compare.astro, blog, SPONSORS, CONTRIBUTING, concepts.mdx) | official docs | **0.9-0.95** | Birebir alıntılarla |
| Faz durumları (SF/FM/FIP/FMR/FMN/FFO/DCLICK) | `.codex/TASKS.md` deterministik `[x]` sayımı | **0.9** | Tek kaynak ama deterministik |
| Karar geçmişi (H bölümü) | `.codex/MEMORY.md` + evidence dosyaları + spec/plan belgeleri | **0.85** | Çoğu için birden fazla belge teyit ediyor |
| Mimari ilke uyumu (runtime/client boundary) | belge kanıtı (4 kaynak, korelasyonlu) | **0.9** | Diff bağımsız çalıştırılmadı |
| **Test/gate sonuçları (3.683/3.683 vb.)** | **korelasyonlu süreklilik belgeleri — bağımsız çalıştırma YOK** | **0.6** | §R8, §I.4-T4 |
| Upstream'in file-manager'ı kapsamaması | negatif grep (README/compare/concepts/CHANGELOG) | **0.8** | Negatif kanıt doğası gereği zayıf |
| Edit yeteneğinin yokluğu | negatif grep + fonksiyon envanteri çapraz-kontrolü | **0.9** | İki yöntem |
| Custom layout B-zincirinin başlamamışlığı | negatif grep + dosya varlık kontrolü | **0.9** | `grep -i "custom layout" .codex/CURRENT.md` → 0; B2 spec dosyası yok |
| `unwrap()` ihlal durumu | kaba grep — **ayrıştırılamadı** | **0.5** | §I.4-T5 |

### K.2 Düzeltilen ölçüm hatası

İlk taramada `find docs -type f | head -80` kesmesi yüzünden `.codex/CURRENT.md`'nin atıf yaptığı beş spec dosyası (`2026-07-19-herdr-files-layout-v1-lock.md`, `...-rapid-navigation-latency-prd.md`, `...-content-locations-rail-design.md`, `2026-07-17-herdr-custom-layout-architecture-guide.md`, `2026-07-15-ratatui-reference-intelligence-v2-1-design.md`) "eksik" sanıldı ve sahte bir anomali olarak işaretlendi. Tam listeleme (`ls -la docs/superpowers/specs/`) hepsinin mevcut olduğunu gösterdi. **Rapordaki tüm dosya-varlık iddiaları düzeltilmiş ölçüme dayanıyor.** Bu vaka §I.4-T2'de tuzak olarak kayda geçirildi.

### K.3 Doğrulanamayan iddialar (dürüstlük kaydı)

1. **Test gate sonuçları** — bu analiz salt-okuma kapsamındaydı; `just check` veya nextest çalıştırılmadı. Tüm test sayıları belge alıntısıdır.
2. **Production `unwrap()` ihlali olup olmadığı** — 461 inline test bloğu kaba grep'i bulandırıyor; proje diff-tabanlı denetim kullanıyor (yöntem sağlam) ama bu turda çalıştırılmadı.
3. **FFO diff'inin gerçekten boş olduğu** (`src/server`, `src/protocol` vb.) — belge iddiası, `git diff` ile bağımsız doğrulanmadı.
4. **Upstream'in gelecekteki niyeti** — yalnızca yayınlanmış belgelerden okundu; maintainer'ın açıklanmamış planı bilinmiyor.

### K.4 Bu belgenin bakımı

- **Bayatlanma sinyali:** `.codex/TASKS.md` `[x]` sayımı değiştiğinde, `git rev-list` divergence sayıları değiştiğinde veya yeni bir faz açıldığında bu belge güncellenmelidir.
- **Güncelleme yöntemi:** §I.2 ve §I.3 komutlarını çalıştır, §D ve §0.2 tablolarını yenile, §H'ye yeni kararları ekle (eski satırları SİLME — durum kolonunu `SÜPERSEDED` yap).
- **Silme yasağı:** Kullanıcı direktifi (2026-07-24): *"hicbir analiz referans projeler kaynaklar falan kessinlikle bosa gidemez silinemez"* — bu belge ve atıf yaptığı kaynaklar kalıcıdır.
- **Git koruması yok:** `docs/analysis/` `.gitignore`'da (`/docs/*`). Makine kopyası için `~/.cartography/herdr-vision-mission-*` yolunu kullan (§R4).

---

*Analiz: 2026-07-24 · Salt-okuma · herdr `feat/native-fm` @ `b48bd903` · graph 24.357/129.892*
