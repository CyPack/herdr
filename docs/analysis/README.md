---
doc: herdr-analysis-hub
domain: _index
created: 2026-07-24
status: canonical — bu dizindeki her analiz (claim, evidence, confidence) sözleşmesine tabidir
git_note: >
  /docs/* herdr .gitignore'da IGNORED (yalnız /docs/next/ istisna) → bu dizin LOKAL yaşar,
  upstream'e/PR'a SIZMAZ (external-contributor guardrail'e bilinçli uyum — docs/references/README.md
  ile aynı politika). Kayıp riskine karşı ZORUNLU çift kopya: ~/.cartography/herdr-analysis/
agentic_triggers:
  - "analiz · durum analizi · nerede kaldık · proje durumu · state analizi"
  - "vizyon · misyon · faz · roadmap · öncelik"
  - "belge render · png · xlsx · pdf · preview · dosya edit"
  - "custom layout · shell template · bölge · region"
  - "mimari · seam · dikiş · katman · protokol"
  - "devir · handover · chat geçmişi · task envanteri"
related:
  - docs/references/README.md      # kaynak registry index
  - docs/patterns/                 # damıtılmış pattern katalogları
  - .cartography/                  # evidence graph'ları
  - .local/CURRENT-HANDOFF.md      # aktif iş durumu (kanonik pointer)
---

# herdr — Analiz & Referans Havuzu

> **Amaç:** Yapılan hiçbir derin araştırma, kaynak taraması, referans proje incelemesi veya
> durum analizi **uçmasın**. Gelecekte benzer bir değerlendirme turuna girildiğinde (yeni ihtiyaç,
> daha fazlası, alternatif çözüm arayışı) buradan **devam edilsin**, sıfırdan başlanmasın.
>
> Kullanıcı direktifi (2026-07-24, birebir): *"tum kaynaklari analizleri hepsini kalici kaynak ve
> dosyalara tasi... hicbir analiz referans projeler kaynaklar falan kessinlikle bosa gidemez
> silinemez!"* · *"project referans havuzumuz feature referans havuuzmuz cok dolu ve genis olmak
> zorunda!!"*

---

## 1. Bu havuzun üç katmanı

| Katman | Dizin | Ne içerir | Ne zaman okunur |
|---|---|---|---|
| **Analiz** | `docs/analysis/` | Bir tarihte yapılmış TAM durum incelemesi — ham bulgular, kanıtlar, tablolar, alıntılar | "Bu konuda ne biliyoruz?" |
| **Referans** | `docs/references/` | Kaynak registry — URL/dosya + tier + confidence + hangi pattern'i desteklediği | "Bu iddianın kaynağı ne?" |
| **Pattern** | `docs/patterns/` | Damıtılmış desen kataloğu — ne · ne zaman KULLAN · ne zaman KULLANMA | "Nasıl yapmalıyım?" |

Ek: `.cartography/*-SYSTEM-MAP.json` — makine-okunur evidence graph'ları (node/edge biçiminde claim+kaynak).

---

## 2. Dosya envanteri (2026-07-24 turu)

Bu tur, altı paralel incelemenin çıktısıdır. Tetikleyen soru: *"vizyon-misyon durumu nasıl + PNG/XLSX/PDF
render-edit + custom layout altyapısı ne durumda?"*

| # | Dosya | Konu | Kimin sorusuna cevap |
|---|---|---|---|
| 1 | `2026-07-24-vision-mission-state.md` | Ürün vizyonu, misyon durumu, faz haritası, fork↔upstream 819-commit ayrışması, 8 stratejik risk, **karar geçmişi arşivi** | "Neredeyiz, nereye gidiyoruz?" |
| 2 | `2026-07-24-chat-forensics-codex-cursor-handover.md` | Codex→Cursor devri, 227 chat mesajı, task envanteri drift'i, **kullanıcı direktifleri arşivi**, chat forensics reçetesi | "Ne konuşuldu, ne devredildi, ne açık kaldı?" |
| 3 | `2026-07-24-architecture-seams.md` | Katmanlar arası dikiş, grafik protokol yolu, genişleme maliyeti tabloları, 13 kırılganlık, **codebase-mcp sınırları** | "Yeni bir şey eklemek neye mal olur?" |
| 4 | `2026-07-24-document-render-internal-state.md` | herdr'ın PNG/XLSX/PDF iç gerçeği, politika engelleri, 13 yeniden kullanılabilir kalıp, **edit giriş kapısı** | "Bugün ne var, ne yok?" |
| 5 | `2026-07-24-document-render-ecosystem.md` | Terminal görsel/spreadsheet/PDF ekosistemi, aday teknolojiler, **reddedilen adaylar + yeniden-açılma koşulları**, karşılaştırma metodolojisi | "Dünyada nasıl yapılıyor?" |
| 6 | `2026-07-24-custom-layout-state.md` | Shell/template/region altyapısı, 26 maddelik tasarım↔kod grid'i, B-chain durumu, **B1 için hazır girdi** | "Custom layout ne durumda?" |
| 7 | `2026-07-24-decision-matrix-and-roadmaps.md` | **Sentez** — 6 bölümün kesişimi, öncelik tablosu, dört seçeneğin ayrı yol haritaları, edit alternatifleri pros/cons | "Ne yapalım, alternatifler ne?" |

### 2026-07-25 turu (upstream senkron + Şerit 1 araştırması)

| # | Dosya | Konu | ⚠️ Kritik bulgu |
|---|---|---|---|
| 8 | `2026-07-25-license-impact-agpl-to-apache.md` | Upstream AGPL-3.0 → Apache-2.0 geçişinin fork'a etkisi | Fork AGPL'de kalıp Apache kodu alabilir (tek yönlü uyumluluk) |
| 9 | `2026-07-25-upstream-merge-recon.md` | 125 commit senkronu, **19 dosyada çakışma**, strateji seçenekleri | Çakışanlar tam aktif alanımız: `ui.rs`, `state.rs`, `kitty_graphics.rs` |
| 10 | `2026-07-25-preview-performance-and-signals.md` | Debounce · uzak kalite · grafik kare aşımı + **yazi karşılaştırması** | 🔴 **PNG önizlemesi server modunda ERİŞİLEMEZ** (`pub(super)` + `App::run` yalnız monolithic) |
| 11 | `2026-07-25-preview-provider-source.md` | `PreviewProviderSet` kaynağı — config vs registry vs hibrit | 🔴 **`action_id` atılıyor** → sağlayıcı seti tek başına etkisiz |

### 2026-08-13 turu (hiyerarşi görünürlüğü)

| # | Dosya | Konu | ⚠️ Kritik bulgu |
|---|---|---|---|
| 13 | `2026-08-13-module-visibility-state.md` | "Oluşturduğum modüller TUI'da görünmüyor" — dört katman (yazma · yükleme · emisyon · boyama) ayrı ayrı ölçüldü | 🔴 **İki bağımsız sessiz katman üst üste**: managed overlay `[[spaces.node]]`'u hiç birleştirmiyordu **ve** header renderer node anahtarını tanımıyordu → forkta hiçbir `[[spaces.node]]` hiç görünmemiş. İkisi de "config: ok" diyordu |

**Bu turun üç düzeltilmiş varsayımı:**
1. ~~"Üyesiz node tasarım gereği çizilmez"~~ → `Job::Node` boş çocukları emit ediyor; kural yalnız *kök* konumu ilgilendiriyor
2. ~~"Yazma bozuk, menü yanlış kaydediyor"~~ → disk kanıtı aksini gösterdi; sub/paralel anahtarları doğru
3. ~~"Girinti yok, o yüzden sub/paralel ayırt edilemiyor"~~ → girinti çalışıyor (`x=0/2/6` ölçüldü); satır zaten hiç çizilmiyordu

**Bu turun kalıcı iki kapısı:** `scripts/managed_overlay_check.py` (model ↔ birleştirme drift'i
build'i düşürür) · `behavior_registry_check.py` bakım-script testlerini pin olarak tanır.

**Bu turun teslim tuzağı:** `cp` ile çalışan binary'nin üzerine yazılamaz ("Text file busy") ama
`live-handoff` yine "complete" der → `cp` + `mv -f` şart, ve kurulan dosyanın boyutu/mtime'ı
build çıktısıyla karşılaştırılmalıdır.

**2026-07-25 turunun dört düzeltilmiş varsayımı** (önceki turda yanlış bilinen):
1. ~~"PNG hazır, bayrağı aç"~~ → server modunda mimari olarak erişilemez
2. ~~"Sağlayıcıyı doldur, PDF/XLSX görünür"~~ → `action_id` `..` ile atılıyor; enum varyantı + render dalı da gerekli
3. ~~"master 742 commit geride"~~ → fork upstream'den **819 önde**, upstream'de **125** yeni commit var
4. ~~"her satırda decode"~~ → 1-derinlikli slot burst'ü ≤2 decode'a indiriyor

Eşlik eden registry ve pattern dosyaları:

| Dosya | İçerik |
|---|---|
| `docs/references/document-rendering.md` | 30+ etiketli kaynak (crates.io, GitHub, protokol spec'leri, herdr iç kaynakları) |
| `docs/references/custom-layout.md` | Layout kaynakları (spec, evidence, kod modülleri, test adları, commit'ler) |
| `docs/patterns/document-rendering.md` | Görsel/tablo/PDF render desenleri + anti-pattern'ler + ölçek matrisi |
| `docs/patterns/custom-layout.md` | Bölge/track/generation/resize desenleri + karar matrisi |
| `docs/references/README.md` | **Domain index** — hangi domain hangi dosyada |

---

## 3. Okuma sırası (yeni gelen agent için)

```
Soru tipi                          → Okuma sırası
──────────────────────────────────────────────────────────────────────────
"Nerede kaldık / ne yapıyorduk?"   → .local/CURRENT-HANDOFF.md → (2) → (1)
"Neden böyle karar verilmiş?"      → (1) §Karar geçmişi arşivi → (2) §E
"Belge render işi yapacağım"       → (4) → (5) → docs/patterns/document-rendering.md → (3) §E-1
"Custom layout işi yapacağım"      → (6) → docs/patterns/custom-layout.md → (3) §E-2
"Yeni bir yüzey ekleyeceğim"       → (3) §E → (6) §F → (4) §E (kalıplar)
"Yeni bir teknoloji araştıracağım" → (5) §Karşılaştırma metodolojisi → docs/references/
"Ne yapmalıyız?"                   → (7) karar matrisi
```

---

## 4. Yeni analiz eklerken (kayıt kuralı)

`~/.claude/rules/reference-registry.md` 5-adım pipeline'ı geçerli:

1. **Analiz dosyası** → `docs/analysis/YYYY-MM-DD-<konu>.md`, frontmatter'da `domain` + `agentic_triggers` + `git_note` ZORUNLU.
2. **Kaynaklar** → `docs/references/<domain>.md` (yoksa oluştur) — her giriş `tier` + `confidence`. Çıplak kaynak YASAK, uydurma URL YASAK, erişilemeyeni `⚠️ doğrulanamadı` işaretle.
3. **Damıtım** → `docs/patterns/<domain>.md` — pattern ID + ne zaman KULLAN/KULLANMA + ölçek matrisi.
4. **Harita** → `.cartography/<domain>-SYSTEM-MAP.json` (generic `SYSTEM-MAP.json` adını KULLANMA — çakışır).
5. **Index** → bu README'nin envanter tablosuna satır ekle + `docs/references/README.md` domain index'ine satır ekle.

**Ek kural (bu havuza özel):** Her analiz dosyası şu üç bölümü İÇERMELİ —
- *"Bu turda İNCELENMEYEN ..."* — kapsam dışı bırakılanlar + neden + ileride hangi soru için bakılmalı
- *"Reddedilen/ertelenen kararlar + yeniden-açılma koşulları"* — hangi kanıt gelirse karar yeniden açılır
- *"Yeniden kullanılabilir reçete"* — bu araştırmanın metodolojisi, gelecek turda kopyalanabilsin

Sebebi: kullanıcı bu havuzu **tek seferlik rapor arşivi değil, sürekli genişleyen bir değerlendirme
altyapısı** olarak istedi.

---

## 5. Kalıcılık ve kayıp koruması

| Katman | Nerede | Not |
|---|---|---|
| Birincil | `<repo>/docs/analysis/`, `references/`, `patterns/` | `/docs/*` **gitignored** — upstream'e sızmaz (fork guardrail'i, bilinçli) |
| Makine kopyası | `~/.cartography/herdr-analysis/` | Repo silinse/worktree temizlense bile yaşar |
| Harita | `<repo>/.cartography/*-SYSTEM-MAP.json` + `~/.cartography/` | Aynı çift-kopya kuralı |

⚠️ **Bilinen risk (vision-mission analizi R4):** Bu havuz git korumasında DEĞİL. `docs/` altındaki
7 dosya (patterns ×3, references ×3, custom-layout-architecture-guide) hâlihazırda takipsiz.
Git'e almak `.gitignore` değişikliği + fork'a commit gerektirir — **kullanıcı kararı**, henüz alınmadı.
Bu karar alınana kadar **makine kopyası zorunludur**.

---

*v1.0.0 — 2026-07-24 · Altı paralel incelemenin (vizyon-misyon · chat forensics · mimari seam ·*
*belge render iç durum · belge render ekosistem · custom layout) kalıcılaştırma turu.*
