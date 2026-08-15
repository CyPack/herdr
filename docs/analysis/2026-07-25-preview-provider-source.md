---
doc: herdr-analysis
domain: document-rendering
subject: PreviewProviderSet üretim kaynağı — config vs plugin registry vs hibrit (Katman A)
created: 2026-07-25
method: doğrudan kaynak okuma (codebase-memory-mcp bu ajana AÇILMADI — grafik teyidi YOK)
status: canonical — her iddia (claim, evidence=dosya:satır, confidence)
git_note: /docs/* gitignored → lokal. Makine kopyası ~/.cartography/herdr-analysis/
agentic_triggers:
  - "PreviewProviderSet · preview provider · plugin adapter · FMR-5 P5"
  - "OptionalPlugin · action_id · TrailDetailPreview plugin varyantı"
  - "plugin manifest · argv · env · HERDR_PLUGIN_ACTION_ID · file_manifest_actions"
related:
  - docs/analysis/2026-07-24-document-render-internal-state.md
  - docs/analysis/2026-07-24-decision-matrix-and-roadmaps.md
---

# Önizleme Sağlayıcı Kaynağı (Katman A) — 2026-07-25

> Koordinatör tarafından agent raporundan yazıldı (agent kesildi). Taban: `b48bd903`.

---

## 🔴 BULGU-1 — sağlayıcı seti TEK BAŞINA ETKİSİZ

```rust
// src/fm/trail_snapshots.rs:709-714 — action_id BURADA ATILIYOR
PreviewCapability::OptionalPlugin { fallback, .. } => match fallback {
//                                            ^^ action_id düşürülüyor
    PreviewFallback::NativeText => TrailDetailPreview::PendingText,
    PreviewFallback::MetadataOnly(reason) =>
        TrailDetailPreview::MetadataOnly(reason.label().to_owned()),
};
```
Ve `TrailDetailPreview` (`:36-45`): `PendingText | Text | Image | MetadataOnly(String) |
Unpreviewable(String)` — **plugin varyantı YOK**.

⇒ `PreviewProviderSet` doldurulsa bile `OptionalPlugin` fallback'iyle **birebir aynı** davranır.
Kod yeşil derlenir, testler geçer, **ekranda hiçbir şey değişmez**. conf **0.97**.

**İş üç parçalı:** ① sağlayıcı kaynağı ② `TrailDetailPreview` plugin varyantı ③ render dalı.
**Zorunlu koruma:** A2-R1 RED testi — yoksa adım "yeşil ama etkisiz" biter.

## 🎁 BULGU-2 — adaptörün yarısı ZATEN CANLI (bağlam menüsünde)

```
InstalledPluginRegistry (state.rs:2385)
 → file_manifest_actions()                api/plugins/mod.rs:729  [enabled+manifest+platform+dedup]
 → FileManagerContextMenuModel::from_action_bar_with_plugins()  state.rs:1002
 → sağ-tık menüsü  input/file_manager.rs:757 → revalidation  input/modal.rs:702
 → plugin_invocation_params()  state.rs:1117 → sync_file_manager_plugin_action()  mod.rs:210-264
 → start_plugin_command()  runtime.rs:16
```
Hazır: `PluginActionContext::File` (`api/schema/plugins.rs:347`) · `PluginInvocationContext.file_paths`
(`:382-386`) · saf platform çözümü (`manifest.rs:483-511`, I/O yok) · deterministik sıra + qualified-id
dedup · **3 test** (`mod.rs:2686`, `:1910`, `:2808`). conf **0.95**.

⇒ Registry'den türetmek **yeni mimari değil, test edilmiş deseni ikizlemek**.

---

## `PreviewPluginProvider` anatomisi

```rust
// src/fm/preview_capability.rs:60-72
pub(crate) struct PreviewPluginProvider { pub action_id: String, pub platform_supported: bool }
pub(crate) struct PreviewProviderSet {   // #[derive(Default)] → hepsi None
    pub markdown: Option<..>, pub documents: Option<..>,
    pub archives: Option<..>, pub media: Option<..>,
}
```

`plugin_or_fallback` karar tablosu (`:180-196`):

| provider | platform_supported | action_id | Sonuç |
|---|---|---|---|
| None | — | — | fallback |
| Some | **false** | — | fallback — **sessiz** |
| Some | true | boş | fallback — **sessiz** |
| Some | true | dolu | `OptionalPlugin{action_id, fallback}` → *ama BULGU-1 ile yine fallback'e düşer* |

Modül saflık sözleşmesi (`:1-5`, birebir): *"never reads the filesystem, checks PATH, **loads
configuration**, spawns a process, or mutates file-manager navigation."*

---

## Üç seçenek — GEREKÇELİ ÖNERİ: **(b) plugin registry, b2-minimal**

| | (a) Config | **(b) Registry** | (c) Hibrit |
|---|---|---|---|
| runtime/client boundary | ✅ | ✅ **en temiz** | ✅ |
| Yeni wire/API/protokol | ❌ gerekmez | ❌ gerekmez | ❌ gerekmez |
| FMR-5 uygunluğu | ⚠️ kısmi ("plugin adapter" der, "config" demez) | ✅ **tam** | ✅ |
| Altyapı hazır mı | kısmen | ✅ **BULGU-2** | kısmen |
| Maliyet | Orta | **En düşük** | En yüksek |
| ⛔ Engel | **`[preview]` bloğu ZATEN VAR** ve "browser preview yerleşimi" demek (`config/model.rs:367-397`) → ad çakışması, kalıcı borç | manifest'te "ben `documents` sağlayıcısıyım" demenin yolu yok | öncelik belirsizliği |

**b1 vs b2:** b1 (`contexts=["file"]`'dan tahmin) → kategori eşlemesi tahmine kalır = **sessiz hata
fabrikası**. **b2-minimal**: manifest'e tek opsiyonel alan (`preview_categories = ["documents"]`).
Yayınlanmış v1 sözleşmesini genişletir → `docs/next/` + `min_herdr_version` disiplini.

**Not:** `plugins.mdx:32-34` (yayınlanmış): *"Runtime action registration ... are not part of plugin v1."*
⇒ Sağlayıcı **çalışma-zamanı kaydıyla ilan edilemez**; ya manifest ya config.

### Boundary değerlendirmesi
Registry `AppState.installed_plugins`'te (`state.rs:11,2385`), FM ile **aynı process**. Sağlayıcı seti
onun saf **projeksiyonu** → paylaşılan runtime gerçeği DEĞİL → server API'sinden yayımlanmamalı.
`PROTOCOL_VERSION` (16) **dokunulmaz**. conf 0.9.

---

## Bağımlılık zinciri

| Adım | İş | Çıktı |
|---|---|---|
| **A1 KARAR** | (b1/b2/c) seçimi · kategori ilanı yolu · `action_id` mı `plugin_id+action_id` mi | Yazılı karar kaydı |
| **A2 BAĞLAMA** | ① `TrailDetailPreview` plugin varyantı **(BULGU-1)** ② `prepare_trail_detail(providers)` ③ `FmState` alanı ④ saf seçici | Ekranda **görünür** ipucu |
| **B1 ÖRNEK** | Gerçek plugin ile uçtan uca | Çalışan referans + VIS baseline |
| **B2 FALLBACK** | platform yok / manifest yok / disabled / komut çöker → **açık** fallback | Fail-closed matrisi |

## Test noktaları (koddan ÖNCE)

- **A2-R1** `optional_plugin_capability_survives_into_trail_detail` — **en kritik**, BULGU-1'i kilitler
- **A2-R2** `prepare_trail_detail_uses_injected_provider_set` — derlenmemeli (RED)
- **A2-R3** `preview_providers_are_derived_from_enabled_file_context_actions` — emsal `mod.rs:2686`
- **A2-R5** `ambiguous_bare_action_id_is_not_offered_as_provider` — `ambiguous_plugin_action` (`mod.rs:571`) önizleme yolunda hiç doğmamalı
- **B1-R2** tek-sefer tetikleme — birebir emsal `file_manager_plugin_intent_uses_existing_command_runtime_once` (`mod.rs:1910`)
- **B2-R1** platform desteksiz → fallback + **görünür sebep** (bugün sessiz)

## Sessiz hata riskleri

| # | Risk | Görünür kılma |
|---|---|---|
| F1 | **Sağlayıcı dolu ama ekran değişmiyor** (BULGU-1) | A2-R1 ZORUNLU; `..` yerine `action_id` bağla → derleyici tüm match'leri zorlar |
| F2 | `platform_supported=false` / boş id → sessiz fallback | Kaynakta reddedileni say/logla |
| F3 | Registry boş çünkü `no_session=true` (`app/mod.rs:260-262`) | Teşhis tablosu |
| F4 | Plugin komutu başarısız → yalnız `tracing::warn!` (`mod.rs:256-262`) | Önizleme panelinde açık hata |
| F5 | Aynı qualified-id → sağlayıcı **sessizce düşer** (`mod.rs:754-756`) | Çakışmayı göster |
| F7 | Uzantı listesi **iki yerde** (`preview_capability.rs:126-136` + `entry_kind.rs:168`) | Birlikte güncelle |

## Açık sorular (kullanıcı kararı)

| # | Soru | Öneri |
|---|---|---|
| G1 | Manifest sözleşmesi genişleyecek mi (`preview_categories`)? | **Evet, tek opsiyonel alan** |
| G2 | `action_id` tek string mi, `plugin_id`+`action_id` çifti mi? | **Çift** — FM menüsü zaten böyle (`state.rs:940-943,1136`) |
| G3 | `herdr plugin install` sonrası registry FM'e canlı yansıyor mu? | **Doğrulanmadı** |
| G7 | Upstream merge önce mi sonra mı? | **Önce merge** — `trail_snapshots.rs`/`preview_capability.rs` çakışma riski |

## ⚠️ Upstream merge sonrası yeniden doğrula (P0)

`trail_snapshots.rs:709-714` `..` atma noktası hâlâ duruyor mu · `PreviewProviderSet::default()` tek
üretim çağrısı mı · `TrailDetailPreview` varyantları · `file_manifest_actions` emsali · FMR-5 P5 hâlâ `[ ]` mi.

## Doğrulanamayanlar
- codebase-memory-mcp grafiği (tool açılmadı) — bulgular yalnız kaynak okumasından
- Test sonuçları (cargo çalıştırılmadı) — test adları okundu, geçtikleri iddia edilmiyor
- `herdr plugin install` → canlı registry tazeleme
- Upstream 125 commit deltası

---
*v1.0.0 — 2026-07-25*
