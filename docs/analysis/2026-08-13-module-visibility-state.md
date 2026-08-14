---
doc: module-visibility-state
domain: hierarchy-n-level
created: 2026-08-13
repo_head: af0221b9
branch: master
status: iki katman İNDİ ve DEPLOY EDİLDİ (c38e804, af0221b) · bir katman AÇIK (L3b)
method: her katman ayrı ölçüldü — disk (cat), yükleme (unit), emisyon (entries testi), boyama
  (TestBackend buffer dökümü), ürün (canlı binary + kullanıcının gerçek config'i). Hiçbir iddia
  okumaya dayanmıyor; iki hipotez ölçümle YANLIŞLANDI ve kayda geçti.
related:
  - behaviors/hierarchy-n-level.md      (TP-MOD-01, TP-MOD-09..12)
  - behaviors/hierarchy-rank-tree.md    (TP-MOVL-01..07)
  - .local/prd/2026-08-12-managed-node-overlay-PRD.md
  - .local/prd/2026-08-13-module-row-visible-PRD.md
  - scripts/managed_overlay_check.py    (yapısal kapı)
---

# Modül Görünürlüğü — Durum Analizi

**Kullanıcı iki kez modül oluşturdu, ikisi de hiç görünmedi. Sebep tek bir hata değil,
ÜST ÜSTE BİNMİŞ ÜÇ SESSİZ KATMAN'dı.** İkisi kapatıldı, biri açık.

```
YAZMA ✅  →  YÜKLEME ❌ (kapatıldı c38e804)  →  EMİSYON ⚠ (kısmi, L3b açık)  →  BOYAMA ❌ (kapatıldı af0221b)
```

Her katman kendi başına "başarılı" raporluyordu. Hiçbir doğrulayıcı bunu yakalayamazdı —
çünkü hiçbir şey *geçersiz* değildi.

---

## 1 · Şikâyet ve ilk yanılgı

> "o kadar hiyerarşik ve sonsuz sub modül / paralel modül oluşturma seçenekleri ekledik, e bunlar
> modül oluşturmuyor. TUI'da görünmüyor oluşturduğum modüller."

İlk akla gelen iki açıklama da **yanlış** çıktı. İkisini de kayda geçiyorum, çünkü sonraki ajanın
aynı yola sapması muhtemel:

| Yanlış hipotez | Nasıl öldü |
|---|---|
| "Üyesiz node tasarım gereği çizilmez, kural bu" | `Job::Node` boş çocukları **emit ediyor** (`sidebar.rs:1317-1328`); `subtree_first_ws` boş alt-ağaca `len()+1` verip sona sıralıyor. Kural yalnız *kök* konumdaki üyesiz kabı ilgilendiriyor |
| "Yazma bozuk, menü yanlış kaydediyor" | `~/.config/herdr/spaces.managed.toml` diskte iki node'u **doğru parent'la** taşıyordu; `modal.rs:1033/1036` sub ve paralel için doğru anahtarı geçiyor |

**Ders:** "görünmüyor" şikâyetinde ilk refleks *çizim* katmanına atlamak olur. Doğru refleks
**zinciri baştan sona ayrı ayrı ölçmek**tir; bu vakada kırık halkalar zincirin iki ayrı ucundaydı.

---

## 2 · Katman katman ölçüm

### 2.1 · YAZMA — sağlam

| Yol | Ne yazıyor | Kanıt |
|---|---|---|
| ⋯ → "New sub-module..." | `parent = Some(node_key)` | `src/app/input/modal.rs:1033` |
| ⋯ → "New parallel module..." | `parent =` düğümün **kendi** parent'ı | `modal.rs:1036-1042` |
| Kova başlığı → sub | `parent = Some(space_key)` (TP-NODE-08) | `modal.rs:1055` |
| Kova başlığı → paralel | `space_owner_for_key` | `modal.rs:1058-1060` |
| `space move --new-group` | node + üyelik tek yazımda | `cli/space.rs` (TP-RANK-10) |

Hepsi `~/.config/herdr/spaces.managed.toml`'a gider. Disk kanıtı:

```toml
[[spaces.node]] key = "group:uzaktan-ses-muzik-film-browser"
                name = "UZAKTAN SES MUZIK FILM -BROWSER"  parent = "project:herdr"
[[spaces.node]] key = "group:uzaktan-ses"
                name = "UZAKTAN SES"                      parent = "project:herdr"
```

### 2.2 · YÜKLEME — kırıktı, kapatıldı (`c38e804`)

```rust
// src/config/io.rs · merge_managed_spaces_str — DÜZELTME ÖNCESİ
config.spaces.split.extend(managed.spaces.split);
config.spaces.project.extend(managed.spaces.project);
// config.spaces.node  ← HİÇ BİRLEŞTİRİLMİYORDU
```

`[[spaces.node]]` N-seviye işinde (#36/#37) modele eklendi, **managed overlay birleştirmesi o gün
genişletilmedi**. TOML başarıyla parse ediliyor, `managed` yerel değişkeninde duruyor, fonksiyon
dönerken çöpe gidiyordu. Diagnostic üretilmiyordu — **çünkü hata yoktu**; sadece kimse o alanı
okumuyordu.

**Ölçüm:** kullanıcının gerçek config'i, `herdr space list` → 20 `rule` + 4 `project` + **0 node**.

### 2.3 · EMİSYON — kısmi (L3b AÇIK)

Emisyon **workspace-güdümlüdür** (`sidebar.rs:1171` `for ws_idx in 0..app.workspaces.len()`).
Bir node'a ancak altındaki bir checkout'tan tırmanılarak varılır.

| Vaka | Durum | Sebep |
|---|---|---|
| Çizilen bir atanın altındaki boş modül | ✅ emit ediliyor | `Job::Node` → `node_children` gezintisi; test `an_empty_module_under_a_drawn_project_takes_a_row_of_its_own` |
| **Top-level** boş modül | ❌ emit EDİLMİYOR | Hiçbir workspace ona tırmanmaz → Job stack'e hiç girmez |
| **Üyesiz kova** altındaki beyanlı modül | ❌ kaybolur | `Job::Bucket` üye yoksa `continue` (`sidebar.rs:1236-1241`) ve **çocuklarını gezmez** |

### 2.4 · BOYAMA — kırıktı, kapatıldı (`af0221b`)

```rust
// src/ui/sidebar.rs · render_workspace_project_headers — DÜZELTME ÖNCESİ
let Some(project) = project_for_key(app, &head.project_key) else {
    continue;                     // ← node anahtarı BURADA düşüyordu
};
```

`project_for_key` (`sidebar.rs:590`) yalnız `app.space_projects`'e bakar; `spaces.projects()`
(`config/model.rs:546`) yalnız `[[spaces.project]]` döndürür. **`node_for_key` (`sidebar.rs:450`)
vardı ve renderer'dan hiç çağrılmıyordu** — tek kullanımı `:497`'de bir yüklem.

Sonuç: satır emit edilir, listede **dikey yer kaplar**, hiç boyanmaz → **boş satır**.

**Ölçüm** (TestBackend, satır dökümü — düzeltme öncesi):

```
 Spaces  Projects   Files
▾ 📁  project:herdr          ← x=0   boyanıyor
  ▾ TUI                      ← x=2   boyanıyor
      · alpha                ← x=6   boyanıyor
      · beta
(UZAKTANSES hiçbir satırda YOK)
```

**Kapsam:** yalnız kullanıcının yeni modülleri değil — config'teki `ccd:field` ve `ccd:quality`
node'ları da. Yani forkta **hiçbir `[[spaces.node]]` hiç görünmemiş**.

### 2.5 · GİRİNTİ — sağlamdı (üçüncü hipotez yanlışlandı)

Yukarıdaki dökümdeki `x=0 / x=2 / x=6` değerleri girintinin çalıştığını gösteriyor. "Sub ile
paralel ayırt edilemiyor" şikâyeti **girinti mekanizmasının yokluğundan değil**, satırın hiç
çizilmemesinden geliyordu. Node satırı için gereken tek şey derinliği **üyeden değil node
zincirinden** hesaplamaktı (`node_depth`) — çünkü yeni kurulmuş modülün üyesi yoktur.

---

## 3 · Kusur sınıfı: "geçerli ama okunmayan" / "yerleştirilmiş ama boyanmayan"

Bu iki kusurun ortak imzası:

```
config: ok   ∧   ekranda hiçbir şey yok   ∧   kullanıcı kendini suçluyor
```

**Hiçbir doğrulayıcı bunları yakalayamaz.** Değer geçerlidir, dosya geçerlidir, `herdr config
check` haklı olarak "ok" der. Yakalayan şey **yapısal kapı**dır.

### 3.1 · Emsal — aynı sınıf daha önce de yaşandı

Kodun kendi yorumu (`sidebar.rs`, `render_workspace_more_chats_rows`):

> *"Before this it was laid out but never painted, so the desktop drawer ended in a blank line
> the reader could neither understand nor act on."*

"Older chats" satırı bir sürüm boyunca aynı şekilde kayıptı. **İki kez tekrarlanan bir kusur
sınıfı artık bir tesadüf değil, mimari bir açıktır:** satır üretimi ile satır boyaması iki ayrı
tabloya bakar ve aralarında hiçbir kapı yoktur.

### 3.2 · Kardeş bulgu (paralel ajan, aynı gün)

`feat/custom-layout` ajanı `herdr config check`'te şunu buldu: **kullanılamaz DEĞER
raporlanmıyor** (`[shell.bars.top] size = 999` → "ok", bar çizilmiyor). Üç değerli yüklemle
düzeltti (`1b60ea37`).

| | o | bu |
|---|---|---|
| sınıf | **doğrulayıcı** eksiği — kural vardı, kimse sormuyordu | **kapsama** eksiği — alan okunuyor, birleştirmede düşüyor |
| çare | yüklem (`ok` / `empty-for-now` / `unusable`) | **yapısal kapı** (model ↔ birleştirme karşılaştırması) |

Dördüncü bir değer eklemek yanlış çözüm olurdu: değer geçerli olduğu için doğrulayıcının söyleyecek
sözü yok.

---

## 4 · Konan kapılar

### 4.1 · `scripts/managed_overlay_check.py` (TP-MOVL-03)

`SpacesConfig`'in her `Vec` alanı `merge_managed_spaces_str` tarafından `extend` edilmek
ZORUNDA; edilmiyorsa ya `EXEMPT_FIELDS`'a gerekçesiyle yazılacak ya da build DÜŞECEK. Ayrıca
çapraz bağlama (`config.spaces.a.extend(managed.spaces.b)`) reddedilir.

**Doğrulama:** düzeltilmemiş kaynağa karşı çalıştırıldığında **gerçek tarihsel hatayı bağımsız
olarak yakaladı** — kapı, düzeltmeyi görmeden yazıldı.

### 4.2 · `behavior_registry_check.py` genişletmesi

Bakım-script testleri artık davranış çivileyebilir. **Yalnız test ADI kaynağı olarak** —
marker taraması genişletilmedi, çünkü ilk denemede `scripts/test_*.py` içindeki fixture dizeleri
(`TP-DOC-01` vb.) marker sanıldı ve 10 sahte "belgesiz davranış" üretti. Ayrım:
**davranış BEYAN etmek kaynak-dosya işidir; ÇİVİLEMEK değildir.**

### 4.3 · Davranış kayıtları

| Aile | Nerede | Ne çiviliyor |
|---|---|---|
| TP-MOVL-01..07 | `behaviors/hierarchy-rank-tree.md` | overlay kapsaması, `space list` konteyner satırları, kaynak etiketi, yalnız-konteyner ağacı |
| TP-MOD-01 | `behaviors/hierarchy-n-level.md` | boş modül çizilen atanın altında satır alır, dolu kovalardan sonra |
| TP-MOD-09..12 | `behaviors/hierarchy-n-level.md` | ad boyanır · sub/paralel girintisi · katlanmış modülde uydurma nokta yok · proje karakterizasyonu |

---

## 5 · Teslim zinciri — ölçülen halkalar ve bir tuzak

`observed-defect-protocol §6` zinciri iki kez yürütüldü:

| halka | 1. teslim (overlay) | 2. teslim (boyama) |
|---|---|---|
| kaynak | `c38e804` | `af0221b` |
| build | release 23:15 | release 07:50 |
| kurulum | 23:16 (yedek `herdr.bak-premovl-2320`) | 07:50 (yedek `herdr.bak-prel3a-2350`) |
| süreç | PID 1547132 → 2221290 | PID → **141023** |
| oturumlar | 24/24 korundu | 24/24 korundu |
| gözlem | `space list` 4 node | kullanıcı onayı bekliyor |

### 5.1 · TUZAK — `cp: Text file busy`

İkinci teslimde `cp target/release/herdr ~/.local/bin/herdr` **başarısız oldu** ("Text file
busy": çalışan binary'nin üzerine yazılamaz). Buna rağmen `live-handoff` "complete" dedi ve PID
değişti. **Ölçülmeseydi "deploy edildi" denecekti** — oysa eski binary yeniden başlamıştı.

```bash
# YANLIŞ
cp target/release/herdr ~/.local/bin/herdr        # Text file busy → SESSİZ BAŞARISIZLIK
# DOĞRU
cp target/release/herdr ~/.local/bin/herdr.new && mv -f ~/.local/bin/herdr.new ~/.local/bin/herdr
```

`mv` dizin girdisini değiştirir, çalışan inode'a dokunmaz. **Kanıt: kurulan dosyanın boyutu ve
mtime'ı, build çıktısıyla karşılaştırılmalıdır** — "komut hata vermedi" yeterli değildir.

---

## 6 · Açık kalan (L3b — task #66)

| # | Vaka | Karar |
|---|---|---|
| a | Top-level boş modül emit edilmiyor | **K3** — beyanlı orman İKİNCİ satır kaynağı olacak: workspace yürüyüşü bittikten sonra emit edilmemiş beyanlı node'lar pre-order, workspace satırlarının ARDINDAN |
| b | Üyesiz kova altındaki beyanlı modül kayboluyor | **K4** — üyesiz TÜREME kova hâlâ başlık üretmez (TP-MOD-02 hayalet başlık yasağı) ama **çocuklarını gezer** |

**Test noktaları hazır:** TP-MOD-13 (top-level satır alır) · TP-MOD-14 (beyanlı satırlar
workspace'lerden sonra) · TP-MOD-15 (üyesiz kova altındaki modül yaşar, kova başlık üretmez) ·
TP-MOD-16 (iç içe boş zincir pre-order) · TP-MOD-17 (çift emisyon yasak) · TP-MOD-18 (node'suz
ağaç birebir aynı) · TP-MOD-19 (katlanmış ata beyanlı çocuğu gizler).

**Sonraki katmanlar:** L4 (boş modül "boşum" der + `+`/`⋯` + sil) · L5 (PWA paritesi).

---

## 7 · Bu analizin bıraktığı üç kalıcı ders

1. **Bir alan modele eklendiğinde, o modeli dolduran HER yol genişletilmelidir.** Yoksa alan bir
   yoldan girer, öbüründen sessizce düşer. Bunu kod incelemesi değil **yapısal kapı** yakalar.
2. **Satır ÜRETMEK ile satır BOYAMAK iki ayrı sorudur.** Emit edilen ama boyanmayan satır, yerini
   kaplar ve hasar gibi okunur. Bu fork'ta iki kez oldu.
3. **Teslim zincirinin her halkası ölçülür.** "Komut hata vermedi" bir halka kanıtı değildir;
   halka kanıtı **artefaktın kimliğidir** (boyut, mtime, `/proc/<pid>/exe`).

*Not: `docs/*` bu depoda `.gitignore`'dadır (yalnız `docs/next/` ve `docs/versions/` izlenir).
Bu dosya yerel bilgi katmanıdır; kalıcılığı depo dışı yedeklemeye bağlıdır.*
