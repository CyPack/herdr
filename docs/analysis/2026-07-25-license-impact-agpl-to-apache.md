---
doc: herdr-analysis
domain: licensing
subject: upstream AGPL-3.0 → Apache-2.0 geçişinin fork'a etkisi
created: 2026-07-25
method: repo gerçekleri (git show/dosya okuma) + lisans metinleri + birincil kaynaklar (WebFetch)
status: canonical — BU HUKUKİ GÖRÜŞ DEĞİLDİR; gerçek derlemesi + belirsizlik kaydı
git_note: >
  /docs/* herdr .gitignore'da IGNORED → lokal yaşar, upstream'e sızmaz.
  Makine kopyası: ~/.cartography/herdr-analysis/
agentic_triggers:
  - "lisans · license · AGPL · Apache · copyleft · relicense"
  - "fork bağımsızlığı · kaynak kapatma · proprietary · türev iş"
  - "upstream merge lisans etkisi · LICENSE dosyası"
related:
  - docs/analysis/2026-07-24-vision-mission-state.md
  - docs/analysis/2026-07-24-decision-matrix-and-roadmaps.md
---

# LİSANS ETKİSİ ANALİZİ — herdr (CyPack fork ↔ ogulcancelik/herdr upstream)

> ⚖️ **BU HUKUKİ GÖRÜŞ DEĞİLDİR.** Bu belgeyi yazan bir avukat değildir. Aşağıdakiler yalnızca
> (a) repo'dan doğrulanabilir teknik gerçekler, (b) lisans metinlerinin kendi ifadeleri (birebir
> alıntı), (c) FSF/ASF gibi kurumların yayınladığı yaygın kabul görmüş yorumlardır. Kesin hukuki
> sonuç ilan edilmemektedir. Nihai karar öncesi açık kaynak lisanslama konusunda uzman bir
> hukukçuya danışılmalıdır.

**Bu analizin sebebi (kullanıcının somut endişesi):**

> *"Öncelikle lisans etkisini incele bakalım — bunlar bizim desteklediğimiz şey; sonra kaynak
> kodunu kapatmak falan isteyeceklerse biz kendimiz hallederiz ya kendi fork'umuzdan!
> Öyle desteklerimizin boşa gitmesine asla müsaade edemeyiz!"*

Yani: **upstream ileride kaynağı kapatırsa (proprietary'ye geçerse), fork'un ve topluluğun
katkıları boşa gider mi? Fork kendi başına devam edebilir mi?**

---

## 0. Yöntem ve Kanıt Sözleşmesi

Her iddia `(claim, evidence, confidence)` üçlüsüyle işaretlendi:

| Kaynak tipi | Örnek | Confidence |
|---|---|---|
| `executable` — lokalde çalıştırılan git/dosya komutu | `git show cd5ea1be` çıktısı | 0.98 |
| `official` — kurumun kendi yayını | gnu.org, apache.org | 0.90 |
| `inference` — iki official kaynaktan zincirleme çıkarım | Apache→AGPL yolu | 0.75 |
| `⚠️ doğrulanamadı` | repo dışı, gözlemlenemeyen | — |

**Variant (V) durumu:** Başlangıçta 10 açık soru vardı. 7'si `verified` oldu, 3'ü repo kanıtıyla
kapatılamaz nitelikte (§H'de listelendi) → V sabitlendi, araştırma döngüsü sonlandırıldı.

**Scope uyumu:** Tüm inceleme salt-okunurdur. Hiçbir git mutasyonu (merge/checkout/commit/push)
yapılmadı, worktree açılmadı, `LICENSE`/`.gitignore` değiştirilmedi, herdr server'a dokunulmadı.
Yazılan tek dosya bu belgedir.

---

## A. OLGU TABLOSU — Tarihsel Zaman Çizelgesi

| Tarih | Commit / Tag | Olay | Lisans durumu | Kanıt |
|---|---|---|---|---|
| 2026-07-08 01:53 | `299dd416` → **v0.7.3** | Upstream release | `AGPL-3.0-or-later` | `git log -1 v0.7.3` (0.98) |
| **2026-07-11 02:46** | **`46174563`** | **ORTAK ATA** — fork buradan ayrıldı (`feat: add copy-on-select setting`) | `AGPL-3.0-or-later` (dual: AGPL + ticari) | `git show 46174563:Cargo.toml` (0.98) |
| 2026-07-15 19:30 | `50aaa2ec` → **v0.7.4** | Upstream release | `AGPL-3.0-or-later` | tag (0.98) |
| 2026-07-21 21:04 | `ef4c23f5` → **v0.7.5** | Upstream release | **`AGPL-3.0-or-later`** ← **son AGPL'li release** | `git show v0.7.5:Cargo.toml` (0.98) |
| **2026-07-22 22:56** | **`cd5ea1be`** | **RELICENSE** — `chore: relicense herdr under apache-2.0` | AGPL-3.0-or-later → **`Apache-2.0`** | `git show cd5ea1be` (0.98) |
| 2026-07-22 … 07-24 | 17 commit | Relicense sonrası upstream işleri | `Apache-2.0` | `git rev-list --count cd5ea1be..upstream/master` = 17 (0.98) |
| 2026-07-23 16:50 | `b48bd903` | **Fork HEAD** (`feat/native-fm`) | **`AGPL-3.0-or-later`** (değişmedi) | `sha256sum LICENSE` (0.98) |

### Kritik olgu: relicense HENÜZ HİÇBİR RELEASE'DE YOK

```
git tag --contains cd5ea1be   →   (BOŞ)
```

`claim:` Relicense commit'i hiçbir tag'de yok; yayınlanmış tüm herdr sürümleri (v0.7.3, v0.7.4,
v0.7.5 ve tüm preview'lar, en yenisi `preview-2026-07-21`) **AGPL-3.0-or-later** altında
dağıtıldı. Apache-2.0 şu an sadece `master` branch'inde, yayınlanmamış durumda.
`evidence:` executable — `git tag --contains cd5ea1be` boş; `git show v0.7.5:Cargo.toml | grep license`
→ `AGPL-3.0-or-later`. `confidence: 0.98`

### İki-kaynak state kıyaslaması (fork ⟷ upstream)

```
  ── herdr lisans state · 2026-07-25 ──  (🗂️ CyPack fork @b48bd903  ·  📋 upstream @38d2b078)
┌─────┬──────────────────────────────────────┬─────┬──────────────────────────────────────────┐
│ #   │ 🗂️ CyPack fork (origin)               │ ⟷  │ 📋 upstream (ogulcancelik/herdr)          │
├─────┼──────────────────────────────────────┼─────┼──────────────────────────────────────────┤
│ 1   │ LICENSE = AGPL-3.0-or-later (671 str)│ ❌  │ LICENSE = Apache-2.0 (201 str)            │
├─────┼──────────────────────────────────────┼─────┼──────────────────────────────────────────┤
│ 2   │ + "Commercial: hey@herdr.dev" dual    │ ❌  │ (dual-license teklifi KALDIRILDI)         │
├─────┼──────────────────────────────────────┼─────┼──────────────────────────────────────────┤
│ 3   │ Cargo.toml license="AGPL-3.0-or-later"│ ❌  │ Cargo.toml license="Apache-2.0"           │
├─────┼──────────────────────────────────────┼─────┼──────────────────────────────────────────┤
│ 4   │ nix: lib.licenses.agpl3Plus           │ ❌  │ nix: lib.licenses.asl20                   │
├─────┼──────────────────────────────────────┼─────┼──────────────────────────────────────────┤
│ 5   │ README rozeti: AGPL-3.0               │ ❌  │ README rozeti: Apache-2.0                 │
├─────┼──────────────────────────────────────┼─────┼──────────────────────────────────────────┤
│ 6   │ NOTICE dosyası: YOK                   │ ✅  │ NOTICE dosyası: YOK                       │
├─────┼──────────────────────────────────────┼─────┼──────────────────────────────────────────┤
│ 7   │ CLA / DCO: YOK                        │ ✅  │ CLA / DCO: YOK                            │
├─────┼──────────────────────────────────────┼─────┼──────────────────────────────────────────┤
│ 8   │ vendor/libghostty-vt: MIT             │ ✅  │ vendor/libghostty-vt: MIT                 │
├─────┼──────────────────────────────────────┼─────┼──────────────────────────────────────────┤
│ 9   │ vendor/portable-pty: MIT              │ ✅  │ vendor/portable-pty: MIT                  │
├─────┼──────────────────────────────────────┼─────┼──────────────────────────────────────────┤
│ 10  │ src/fm/natsort.rs (MIT, yazi kökenli) │ ❓  │ (upstream'de bu dosya YOK — fork-only)    │
├─────┼──────────────────────────────────────┼─────┼──────────────────────────────────────────┤
│ 11  │ 819 commit, %100 CyPack yazarlı       │ ❓  │ (upstream'de yok)                         │
├─────┼──────────────────────────────────────┼─────┼──────────────────────────────────────────┤
│ 12  │ (fork'ta yok)                         │ ❌  │ 17 commit relicense SONRASI (Apache)      │
└─────┴──────────────────────────────────────┴─────┴──────────────────────────────────────────┘
  Açıklama: ✅ aynı  ❌ farklı/eksik  ❓ yalnız o tarafta
```

---

## B. RELICENSE COMMIT'İ ANALİZİ (`cd5ea1be`)

**Tam commit metadata:**

```
cd5ea1be0e69ed49b6f32f7ed5b333f6c8526874
Ogulcan Celik <ogulcancelik@gmail.com>
Wed Jul 22 22:56:57 2026 +0300

chore: relicense herdr under apache-2.0
```

### ⚠️ GEREKÇE BELİRTİLMEMİŞ

`claim:` Commit mesajında **gövde (body) yoktur** — sadece tek satırlık konu satırı var.
Relicense'ın *nedeni* commit'te açıklanmamış.
`evidence:` executable — `git log -1 --format='%B' cd5ea1be` çıktısı tam olarak tek satır.
`confidence: 0.98`

Repo genelinde de gerekçe aranmıştır:

```
git grep -niE 'contributor license|grant .*licen|assign .*copyright|inbound|relicens' upstream/master
  → TEK SONUÇ: docs/next/CHANGELOG.md:6: "Relicensed Herdr from AGPL-3.0-or-later to Apache-2.0."
```

`claim:` Repo'nun hiçbir yerinde (CONTRIBUTING.md, README, docs, .github) relicense gerekçesi,
duyurusu veya katkı sahiplerinden onay alındığına dair bir iz yoktur. CHANGELOG kaydı da yalnızca
olayı bildirir, gerekçelendirmez.
`evidence:` executable — `git grep` tam ağaç taraması.
`confidence: 0.95` (repo dışı kanal — Discord/Discussions/website blog — kontrol edilmedi, ⚠️)

### Değişen 6 dosya

| Dosya | Değişim | Satır |
|---|---|---|
| `LICENSE` | AGPL-3.0 tam metni (671 satır) → Apache-2.0 tam metni (201 satır) | +210 / −687 |
| `Cargo.toml` | `license = "AGPL-3.0-or-later"` → `license = "Apache-2.0"` | 1 satır |
| `README.md` | Rozet + 6 satırlık dual-license bölümü → tek satır Apache | −6 / +1 |
| `docs/next/README.md` | (aynısı) | −6 / +1 |
| `docs/next/CHANGELOG.md` | `### Changed` / `- Relicensed Herdr from AGPL-3.0-or-later to Apache-2.0.` | +3 |
| `nix/package.nix` | `lib.licenses.agpl3Plus` → `lib.licenses.asl20` | 1 satır |

### Silinen ticari-lisans teklifi (dikkat çekici)

Eski LICENSE ve README'nin başındaki şu blok **tamamen kaldırıldı**:

```
Herdr is dual-licensed:
1. Open source: GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later).
2. Commercial: commercial licenses are available for organizations that cannot comply
   with AGPL. Contact hey@herdr.dev for details.
```

Yerine tek satır geldi: `Herdr is licensed under the [Apache License 2.0](LICENSE).`

**Yorum (teknik, hukuki değil):** Bu değişim, klasik "open-core / AGPL + ticari satış" modelinin
**terk edilmesi** anlamına gelir. AGPL'in ticari değeri, kurumsal kullanıcıyı ya kaynak açmaya ya
ticari lisans satın almaya zorlamasıydı. Apache-2.0'da bu kaldıraç yoktur — kimse ticari lisans
satın almak zorunda kalmaz. Yani hareket, yüzeyde **daha izin verici (permissive)** yöndedir,
daha kapalı yönde değil.

### 🔎 Kaydedilmesi gereken kusur: doldurulmamış telif bildirimi + NOTICE yok

Yeni `LICENSE` dosyası Apache-2.0'ın **ham şablonudur**; Appendix'te şunu içerir:

```
Copyright [yyyy] [name of copyright owner]
```

Yani yıl ve telif sahibi **doldurulmamış**. Ayrıca:

```
find . -iname 'NOTICE*'                       →   (BOŞ)
git ls-tree upstream/master | grep -i notice  →   (BOŞ)
```

`claim:` Upstream'de `NOTICE` dosyası yok ve LICENSE'daki telif satırı boş bırakılmış. Kaynak
ağacında da yalnızca 1 dosyada `Copyright` başlığı var (`src/fm/natsort.rs`, o da fork-only).
`evidence:` executable — `find` + `git ls-tree` + `grep -rl 'Copyright' src/` → 1 sonuç.
`confidence: 0.98`

Bu, Apache-2.0 §4(c)/(d)'nin attribution mekanizmasının pratikte **kurulmamış** olduğu anlamına
gelir — yani kimin telif sahibi olduğu dosya düzeyinde beyan edilmemiş. Aşağı akış (fork dahil)
için bu bir belirsizlik kaynağıdır.

---

## C. CLA / DCO DURUMU — **YOK**

`claim:` Repo'da hiçbir Contributor License Agreement (CLA), Developer Certificate of Origin (DCO),
telif devri sözleşmesi veya `Signed-off-by` zorunluluğu **yoktur**.
`evidence:` executable —

```
grep -rniE 'contributor license agreement|\bCLA\b|\bDCO\b|signed-off-by|developer certificate' \
     --include='*.md' --include='*.yml' --include='*.yaml' --include='*.toml' .   →  0 sonuç
git grep -niE 'contributor license|\bCLA\b|\bDCO\b|signed-off' upstream/master -- .github/   →  0 sonuç
```

`confidence: 0.97`

### Peki `approve-contributor.yml` ne yapıyor?

İncelendi (`.github/workflows/approve-contributor.yml`, 80+ satır JS). Yaptığı **tek şey**:

- Bir maintainer bir issue'ya `/approve @kullanıcı` yazarsa,
- workflow o kullanıcıyı `.github/APPROVED_CONTRIBUTORS` dosyasına ekler,
- bu da `pr-gate.yml`'ın o kişinin PR açmasına izin vermesini sağlar.

`claim:` Bu mekanizma bir **PR açma izni kapısıdır**, telif hakkı devri veya lisans onayı
mekanizması **değildir**. Onaylanan kullanıcı hiçbir lisans metnini kabul etmez, hiçbir hak
devretmez.
`evidence:` executable — workflow kaynak kodu tam okundu; `getCollaboratorPermissionLevel` →
`APPROVED_CONTRIBUTORS` dosyasına isim ekleme dışında işlem yok. `confidence: 0.95`

`.github/APPROVED_CONTRIBUTORS` dosyasında **38 kullanıcı adı** listeli (Edmund-a7, othavioquiliao,
edheltzel, fbettag, reobin, chenrui333, dmmulroy, DeevsDeevs, …).

### Katkı sahipliği tablosu

```
git shortlog -sne upstream/master   →  1207 commit toplam
   1005  Ogulcan Celik <ogulcancelik@gmail.com>     (maintainer)
     17  Can Celik <ogulcancelik@gmail.com>          (aynı kişi, farklı isim)
     53  kangal-bot                                  (bot)
     27+17 github-actions[bot]                       (bot)
     23  akbash <akbash@herdr.dev>                   (proje bot/agent hesabı)
   ────  ve DİĞER 46 AYRI DIŞ KATKI SAHİBİ
```

`claim:` Upstream tarihçesinde maintainer ve botlar dışında **46 farklı dış katkı sahibi** vardır
(Dillon Mulroy, Robin Gagnon, Rui Chen, Zack Drach, Enrico Carlesso, Franz Bettag, …). Bunların
kahir ekseriyeti relicense'tan (2026-07-22) **ÖNCE** katkı yapmıştır — yani kodları
**AGPL-3.0-or-later altında** teslim edilmiştir.
`evidence:` executable — `git shortlog -sne upstream/master | grep -viE 'ogulcancelik|kangal-bot|github-actions|akbash' | wc -l` = 46;
relicense sonrası sadece 2 dış katkı var (`corrius`, `Enrico Carlesso`). `confidence: 0.95`

### Bunun anlamı (mekanizma tespiti — hukuki hüküm DEĞİL)

CLA olmayan projelerde yaygın kabul gören uygulama şudur: katkı sahibi telif hakkını **kendinde
tutar**, kodunu projenin o anki lisansı altında ("inbound = outbound") verir. Bu durumda tek
taraflı relicense için normalde her katkı sahibinin izni gerekir.

⚠️ **Bunun hukuken geçerli olup olmadığına bu belge karar veremez.** Sadece şu raporlanmaktadır:
**repo'da, dış katkı sahiplerinden relicense onayı alındığına dair hiçbir kayıt yoktur.** Bu,
projenin dışında (e-posta, Discord, GitHub Discussions) yapılmış olabilir — repo'dan görülemez
(`⚠️ doğrulanamadı`).

Bu belirsizlik **fork için doğrudan bir risk değildir** (§F'de açıklanıyor), çünkü fork'un
elindeki AGPL kopyası bu tartışmadan bağımsız olarak geçerlidir.

---

## D. AGPL-3.0 ↔ Apache-2.0 KARŞILAŞTIRMA TABLOSU

| Boyut | AGPL-3.0-or-later (fork'un mevcut hali) | Apache-2.0 (upstream'in yeni hali) |
|---|---|---|
| **Copyleft gücü** | **Güçlü (strong copyleft)** — türev işler de AGPL olmak zorunda | **Yok (permissive)** — türev iş herhangi bir lisansla dağıtılabilir |
| **Ağ kullanımı (SaaS deliği)** | **KAPALI** — §13: değiştirilmiş sürümü ağ üzerinden sunuyorsan kaynağı sunmak ZORUNDASIN | **AÇIK** — SaaS olarak sunulan değişiklikler için kaynak açma yükümlülüğü YOK |
| **Patent hükmü** | §11 var (GPLv3 patent hükmü) | **§3 açık patent grant** + patent davası açarsan lisans sona erer |
| **Türev iş yükümlülüğü** | Türev işin **tamamı** AGPL altında dağıtılmalı, kaynak kod verilmeli | Sadece: lisans kopyası ver (§4a), değişen dosyalara not düş (§4b), telif/patent/atıf bildirimlerini koru (§4c), NOTICE varsa taşı (§4d) |
| **🔴 KAYNAĞI KAPATMA İMKÂNI** | **HAYIR** — türev işi proprietary yapamazsın | **EVET** — Apache-2.0 kodu alıp kapalı kaynak ürüne koyabilirsin, kaynak açman gerekmez |
| **Ticari lisans satma kaldıracı** | Var (kurumsal kullanıcı ya uyar ya satın alır) | **Yok** (kimse satın almak zorunda değil) |
| **Aşağı akış hak devri** | §10: alıcı doğrudan orijinal lisans verenlerden lisans alır | §2/§3: alıcı her Contributor'dan doğrudan lisans alır |
| **Geri alınabilirlik** | **Geri alınamaz** (§2, aşağıda alıntı) | **Geri alınamaz** ("irrevocable", §2/§3) |
| **Ek kısıtlama koyma** | §10: "You may not impose any further restrictions" — **YASAK** | §4 son paragraf: kendi değişikliklerine ek/farklı şartlar koyabilirsin |

### Lisans metinlerinden BİREBİR ALINTILAR

**AGPL-3.0 §13 — Remote Network Interaction** (fork'un `LICENSE` dosyası, satır 553-572):

> "Notwithstanding any other provision of this License, if you modify the Program, your modified
> version must prominently offer all users interacting with it remotely through a computer network
> (if your version supports such interaction) an opportunity to receive the Corresponding Source of
> your version by providing access to the Corresponding Source from a network server at no charge,
> through some standard or customary means of facilitating copying of software."

> "Notwithstanding any other provision of this License, you have permission to link or combine any
> covered work with a work licensed under version 3 of the GNU General Public License into a single
> combined work, and to convey the resulting work. The terms of this License will continue to apply
> to the part which is the covered work, but the work with which it is combined will remain governed
> by version 3 of the GNU General Public License."

**AGPL-3.0 §2 — Basic Permissions** (KULLANICININ KORKUSUNA DOĞRUDAN CEVAP):

> "All rights granted under this License are granted for the term of copyright on the Program, and
> are **irrevocable** provided the stated conditions are met."

**AGPL-3.0 §10 — Automatic Licensing of Downstream Recipients:**

> "Each time you convey a covered work, the recipient automatically receives a license from the
> original licensors, to run, modify and propagate that work, subject to this License."

> "**You may not impose any further restrictions** on the exercise of the rights granted or affirmed
> under this License."

**Apache-2.0 §3 — Grant of Patent License** (upstream `LICENSE`):

> "each Contributor hereby grants to You a perpetual, worldwide, non-exclusive, no-charge,
> royalty-free, **irrevocable** (except as stated in this section) patent license to make, have made,
> use, offer to sell, sell, import, and otherwise transfer the Work… If You institute patent
> litigation against any entity (including a cross-claim or counterclaim in a lawsuit) alleging that
> the Work or a Contribution incorporated within the Work constitutes direct or contributory patent
> infringement, then any patent licenses granted to You under this License for that Work shall
> terminate as of the date such litigation is filed."

**Apache-2.0 §4 — Redistribution (yükümlülükler):**

> "(a) You must give any other recipients of the Work or Derivative Works a copy of this License; and
> (b) You must cause any modified files to carry prominent notices stating that You changed the files; and
> (c) You must retain, in the Source form of any Derivative Works that You distribute, all copyright,
> patent, trademark, and attribution notices from the Source form of the Work…
> (d) If the Work includes a "NOTICE" text file… any Derivative Works that You distribute must include
> a readable copy of the attribution notices…"

> "You may add Your own copyright statement to Your modifications and **may provide additional or
> different license terms and conditions** for use, reproduction, or distribution of Your
> modifications, or for any such Derivative Works as a whole…"

**Apache-2.0 §5 — Submission of Contributions:**

> "Unless You explicitly state otherwise, any Contribution intentionally submitted for inclusion in
> the Work by You to the Licensor shall be under the terms and conditions of this License, without
> any additional terms or conditions."

*(Not: Apache-2.0 §5 bir "inbound=outbound" hükmüdür ama **relicense yetkisi vermez** — sadece
katkının Apache-2.0 altında geldiğini söyler. Relicense öncesi AGPL dönemindeki katkılar için
geçerli değildir.)*

---

## E. UYUMLULUK YÖNÜ — TEK YÖNLÜ

### Resmî kaynaklar

**Free Software Foundation — gnu.org/licenses/license-list.html** (`official`, 0.90):

> Apache License 2.0 hakkında: *"This is a free software license, compatible with version 3 of the
> GNU GPL."*
> Ayrıca: *"Please note that this license is not compatible with GPL version 2, because it has some
> requirements that are not in that GPL version."*

**Apache Software Foundation — apache.org/licenses/GPL-compatibility.html** (`official`, 0.90):

> *"Apache 2 software can therefore be included in GPLv3 projects, because the GPLv3 license accepts
> our software into GPLv3 works."*
> *"However, GPLv3 software cannot be included in Apache projects."*

**FSF GPL FAQ — gnu.org/licenses/gpl-faq.html** (`official`, 0.90):

> "In what ways can I link or combine AGPLv3-covered and GPLv3-covered code?" →
> *"Each of these licenses explicitly permits linking with code under the other license. You can
> always link GPLv3-covered modules with AGPLv3-covered modules, and vice versa."*
> *"The copyright holder for a program can release it under several different licenses in parallel."*
> *"If you are the copyright holder for the code, you can release it under various different
> non-exclusive licenses at various times."*

### Sonuç: uyumluluk oku tek yönlüdür

```
   Apache-2.0  ──────►  GPLv3 / AGPLv3        ✅ İZİNLİ
   (permissive kod copyleft projeye girer)

   AGPL-3.0    ──✗───►  Apache-2.0            ⛔ İZİNLİ DEĞİL
   (copyleft kod permissive projeye giremez)
```

`claim:` **Upstream'in yeni Apache-2.0 kodu, fork'un AGPL-3.0 ağacına alınabilir.** Tersi mümkün
değildir: fork'un AGPL kodu upstream'in Apache-2.0 ağacına giremez.
`evidence:` FSF license-list (official, 0.9) + ASF GPL-compatibility (official, 0.9) — iki bağımsız
kurum, aynı yön. θ-kuralı karşılandı → `verified`.
`confidence: 0.90` (Apache→GPLv3 için) · `confidence: 0.75` (Apache→**AGPL**v3 için — FSF tek
cümlede bunu açıkça söylemiyor; çıkarım zinciri: AGPLv3 = GPLv3 + §13 ve FSF ikisinin karşılıklı
birleştirilebilir olduğunu belirtiyor. `⚠️ tek cümlelik resmî beyan bulunamadı`)

**Fork için pratik anlamı:** Bu asimetri fork'un **lehinedir**. Fork AGPL'de kalırsa upstream'den
kod alabilir; upstream fork'tan kod alamaz (fork'un AGPL katkılarını Apache-2.0 ağacına koyamaz —
ancak CyPack izin verirse/yeniden lisanslarsa).

---

## F. FORK'UN SEÇENEK UZAYI — 4 SENARYO

### Seçenek (a) — Fork AGPL-3.0'da KALIR ✅ MÜMKÜN

**Dayanak:**

- Fork'un elindeki kod (`46174563` ve öncesi) AGPL-3.0-or-later altında **geri alınamaz** şekilde
  lisanslandı — AGPL §2: *"irrevocable provided the stated conditions are met"*.
- Upstream'in yeni Apache-2.0 kodu, Apache→GPLv3 tek-yön uyumluluğu sayesinde AGPL ağacına dahil
  edilebilir (FSF + ASF).

**Pratik sonuç:**

- Fork bugünkü haliyle **hiçbir şey yapmadan** AGPL-3.0-or-later olarak devam edebilir.
- Upstream'in relicense sonrası 17 commit'i (ve gelecekteki Apache-2.0 kodu) fork'a merge
  edilebilir; birleşik iş **AGPL-3.0 altında** dağıtılır.
- Karşılığında Apache §4 yükümlülükleri yerine getirilmeli: lisans kopyası, değişen dosyalara not,
  telif/atıf bildirimlerinin korunması.

**Belirsizlik:**

- ⚠️ Apache-2.0 → AGPLv3 (GPLv3 değil) yönü için tek cümlelik resmî FSF beyanı bulunamadı
  (confidence 0.75). Pratikte yaygın kabul görür ama hukukçu teyidi değerlidir.
- ⚠️ Fork'taki `LICENSE` ve `README`, upstream'in **ticari lisans teklifini** (`hey@herdr.dev`) hâlâ
  aynen taşıyor. CyPack o kodun telif sahibi olmadığı için ticari lisans **satamaz**. Bu, halka açık
  bir fork'ta yanıltıcı olabilir → temizlenmesi önerilir (housekeeping, hukuki hüküm değil).

### Seçenek (b) — Fork Apache-2.0'a GEÇER ⚠️ KISMEN MÜMKÜN, ENGELLİ

**Dayanak:**

- FSF GPL FAQ: *"If you are the copyright holder for the code, you can release it under various
  different non-exclusive licenses at various times."*
- CyPack, **kendi 819 commit'inin telif sahibidir** (`git shortlog -sne 46174563..HEAD` →
  `819 CyPack <01cypack@gmail.com>`, %100). Bu katkıları istediği lisansla verebilir.

**AMA — kritik engel:**

- Fork'un tabanı (`46174563` ve öncesi = upstream'in ~1000+ commit'i) **başkalarının telifidir** ve
  fork'a **AGPL altında** ulaşmıştır.
- AGPL §10: *"You may not impose any further restrictions"* — ama tersine, **daha az kısıtlayıcı**
  yapmak da CyPack'in yetkisinde değildir; o kodu Apache-2.0 olarak yeniden lisanslama hakkı telif
  sahiplerine aittir.
- Fork'un ancak **upstream'in yeni Apache-2.0 master'ını temel alarak** yeniden inşa edilmesi
  halinde tamamı Apache-2.0 olabilir — bu, 819 commit'in Apache-2.0 tabanına yeniden uygulanması
  demektir (teknik olarak zahmetli ama mümkün: upstream master AGPL→Apache dönüşümünü zaten yaptı).

**Kim karar verir:** CyPack sadece kendi 819 commit'i için. Upstream kod tabanı için upstream
(+ 46 dış katkı sahibi) karar verir.

**Belirsizlik:** ⚠️ Upstream'in kendi relicense'ının 46 dış katkı sahibi açısından geçerliliği
repo'dan doğrulanamıyor (§C). Fork upstream'in Apache-2.0 master'ını temel alırsa, bu belirsizliği
de **devralır**. Bu, seçenek (a)'nın (AGPL'de kalmanın) gizli bir avantajıdır: fork'un AGPL tabanı
bu tartışmadan etkilenmez.

### Seçenek (c) — 🔴 UPSTREAM KAYNAĞI KAPATIRSA (kullanıcının asıl korkusu)

**Cevap: Elimizdeki kod ELİMİZDE KALIR. Katkılarımız BOŞA GİTMEZ.**

**Dayanak — AGPL-3.0 §2 (birebir):**

> "All rights granted under this License are granted for the term of copyright on the Program, and
> are **irrevocable** provided the stated conditions are met."

**Dayanak — AGPL-3.0 §10:**

> "Each time you convey a covered work, the recipient automatically receives a license from the
> original licensors… **You may not impose any further restrictions** on the exercise of the rights
> granted."

**Dayanak — Apache-2.0 §2/§3:** aynı şekilde `"perpetual… irrevocable"` ifadesi geçer.

**Pratik sonuç — hangi sürüme kadar elimizde kalır:**

| Kod bölümü | Elimizde kalan lisans | Geri alınabilir mi? |
|---|---|---|
| `46174563` ve öncesi tüm upstream kodu (fork tabanı) | **AGPL-3.0-or-later** | **HAYIR** — §2 "irrevocable" |
| v0.7.3 / v0.7.4 / v0.7.5 releaseleri | **AGPL-3.0-or-later** | **HAYIR** |
| `cd5ea1be` ve sonrası upstream kodu (17 commit + gelecek) | **Apache-2.0** — bir kez yayınlandı | **HAYIR** — §2 "perpetual… irrevocable" |
| CyPack'in 819 commit'i | **CyPack'in kendi telifi** — istediği lisansı verir | Upstream'in söz hakkı **YOK** |

**Yani somut olarak:**

1. Upstream yarın repo'yu private yapıp proprietary'ye geçse bile, **bugüne kadar yayınlanmış her
   commit** ilgili lisansı altında (AGPL veya Apache) kalıcı olarak kullanılabilir. Lisans geri
   alınamaz.
2. CyPack fork'u zaten **tam git tarihçesine sahip** (1207 upstream commit + 819 kendi commit'i,
   lokal diskte). Upstream repo'yu silse bile bu kayıp değildir.
3. Upstream'in kapatma sonrası yazacağı **YENİ** kod elimize geçmez — ama o zaten henüz yazılmamış
   koddur, "boşa giden katkı" değildir.
4. `CyPack/herdr` deposu **halka açıktır** ve GitHub üzerinde `ogulcancelik/herdr`'den fork olarak
   görünmektedir (`evidence:` WebFetch github.com/CyPack/herdr, `confidence: 0.85`) — yani dağıtım
   zaten gerçekleşmiş durumdadır.

**Ek koruma — tek yönlü uyumluluğun bize sağladığı:** Fork AGPL'de kalırsa, CyPack'in 819 commit'i
**AGPL altında** dağıtılır. Bu durumda upstream (Apache-2.0'a geçmiş olan) o kodu kendi ağacına
**alamaz** (AGPL → Apache yasak). Yani AGPL'de kalmak, fork'un katkılarının proprietary bir ürüne
süzülmesine karşı **aktif bir kalkandır**.

**Belirsizlik:** ⚠️ "Upstream kaynağı kapatabilir mi?" sorusunun kendisi — upstream'in bunu yapıp
yapamayacağı, 46 dış katkı sahibinin telif durumuna bağlıdır ve **fork'un sorunu değildir**. Fork'un
sorusu "elimizdeki gider mi" ve cevabı lisans metnine göre **hayır**'dır.

### Seçenek (d) — BAĞIMSIZ DEVAM (hard fork)

**Teknik olarak gereken:**

1. **Tam tarihçe elde** — ✅ zaten var: 1207 upstream + 819 CyPack commit lokal repo'da.
2. **Bağımlılık bağımsızlığı** — ✅ doğrulandı (§H "Bağımlılık lisansları"): 301 crate'in tamamı
   permissive; hiçbiri copyleft-only değil. Vendored `libghostty-vt` (MIT, Ghostty) ve
   `portable-pty` (MIT, Wez Furlong) ağaçta fiziksel olarak mevcut.
3. **Marka/isim** — ⚠️ "herdr" ismi ve `herdr.dev` domaini upstream'e aittir. Apache-2.0 §6:
   *"This License does not grant permission to use the trade names, trademarks, service marks, or
   product names of the Licensor"*. AGPL'de de ayrı bir marka izni yoktur. **Bağımsız devam
   senaryosunda proje adı değiştirilmelidir.** Bu, teknik değil marka konusudur ve hukukçu alanıdır.
4. **Altyapı** — sponsorluk (`.github/FUNDING.yml: github: ogulcancelik`), `SPONSORS.md`, website
   (`herdr.dev`), release kanalları (`website/latest.json`, `preview.json`) upstream'e bağlıdır;
   hard fork'ta kendi altyapısı kurulmalıdır.
5. **Lisans** — fork AGPL'de kalabilir (seçenek a). Kendi `LICENSE` başına CyPack telif satırı
   eklenebilir (upstream'in telif satırı korunarak).

**Lisans açısından gereken:** Hard fork için **lisanstan kaynaklı hiçbir engel yoktur**. AGPL ve
Apache-2.0 her ikisi de fork'a, değiştirmeye ve dağıtmaya açıkça izin verir.

---

## G. MERGE KARARINA ETKİSİ — ⚠️ AKTİF KARAR NOKTASI

125 commit'lik upstream merge'i planlanıyorsa, **lisans dosyaları kritik bir karar noktasıdır.**

### Üç-yollu merge'in matematiği (kanıtlı, tahmin değil)

| Dosya | Ortak ata (`46174563`) | Bizim (fork HEAD) | Onların (upstream) | **Merge sonucu** |
|---|---|---|---|---|
| `LICENSE` | AGPL (sha `a7fa24f7…`) | AGPL (**değişmemiş**, sha aynı) | Apache-2.0 | **Apache-2.0** (sessizce, ÇAKIŞMASIZ) |
| `Cargo.toml:7` | `AGPL-3.0-or-later` | `AGPL-3.0-or-later` (**değişmemiş**) | `Apache-2.0` | **Apache-2.0** (sessizce) |
| `README.md` license bölümü | dual-license | **değişmemiş** | tek satır Apache | **Apache** (sessizce) |
| `nix/package.nix` | `agpl3Plus` | **değişmemiş** | `asl20` | **asl20** (sessizce) |

`claim:` Fork bu 4 dosyanın **hiçbirini** ortak atadan bu yana değiştirmemiştir. Git üç-yollu merge
kuralı gereği (`base == ours`, `theirs` değişmiş) **çakışma olmadan, sessizce upstream'in Apache-2.0
sürümü alınır.**
`evidence:` executable —

```
git diff --stat 46174563 HEAD -- LICENSE README.md nix/package.nix   →  (BOŞ, hiç değişiklik yok)
git show 46174563:Cargo.toml | sed -n '7p'  →  license = "AGPL-3.0-or-later"
sed -n '7p' Cargo.toml                       →  license = "AGPL-3.0-or-later"   (AYNI)
sha256sum: 46174563:LICENSE == HEAD:LICENSE  →  a7fa24f7…  (BİREBİR AYNI)
```

`confidence: 0.95`

### 🚨 Bu ne demek

**Merge yapılırsa ve özel bir önlem alınmazsa, fork sessizce AGPL-3.0'dan Apache-2.0'a geçer.**
Uyarı çıkmaz, çakışma çözülmez, kimse fark etmez — sadece `LICENSE` dosyası 671 satırdan 201 satıra
iner.

### İki yol

| | Yol 1: LICENSE'ı MERGE ET (Apache-2.0'a geç) | Yol 2: LICENSE'ı KORU (AGPL'de kal) |
|---|---|---|
| **Nasıl** | Hiçbir şey yapma — varsayılan davranış | Merge sonrası 4 dosyayı geri al: `LICENSE`, `Cargo.toml:7`, `README.md` license bölümü, `nix/package.nix` |
| **Sonuç** | Fork Apache-2.0 olur → CyPack'in 819 commit'i de Apache-2.0 altında dağıtılır → **herkes (upstream dahil) bunları proprietary üründe kullanabilir** | Fork AGPL kalır → CyPack katkıları copyleft korumalı → upstream bunları Apache ağacına **alamaz** |
| **Kaynağı kapatma koruması** | ❌ Yok | ✅ Var (SaaS deliği de kapalı, §13) |
| **Upstream'le kod alışverişi** | ✅ İki yönlü | ⚠️ Tek yönlü (upstream→fork alınabilir; fork→upstream alınamaz) |
| **Lisans uyumu** | ✅ Sorunsuz (Apache tabanı + Apache katkı) | ✅ Sorunsuz (Apache→AGPL izinli, §E) |
| **Ek yükümlülük** | Apache §4 (notice/değişiklik notu) | Apache §4 (upstream kısımlar için) + AGPL §13 (ağ kullanımı) |
| **⚠️ Devralınan risk** | Upstream'in CLA'sız relicense'ının geçerliliği (§C) fork'a da yansır | Fork'un AGPL tabanı bu tartışmadan **etkilenmez** |

### Öneri (teknik, hukuki değil)

Kullanıcının açıkça beyan ettiği öncelik — *"destekleri boşa gitmesin, kaynak kapatılamasın"* —
**Yol 2 (AGPL'de kal)** ile örtüşür. Bu, hem CyPack katkılarını copyleft koruması altında tutar,
hem upstream'den kod almayı engellemez, hem de upstream'in relicense belirsizliğini devralmaz.

**Uygulama notu:** Merge kararı verilmeden önce bu 4 dosya için açık bir karar alınmalı ve merge
komutundan sonra doğrulama yapılmalı:

```bash
sha256sum LICENSE   # a7fa24f74382fb3e4d320a608533a7c2999dbc0f780f1f734c8b891b31f0d9bd = AGPL korundu
grep '^license' Cargo.toml
```

---

## H. BAĞIMLILIK LİSANSLARI + BELİRSİZLİKLER

### Bağımlılık lisansları — ENGEL YOK ✅

#### Doğrudan bağımlılıklar (28 crate)

Lokal cargo registry cache'inden **gerçek `Cargo.toml` dosyaları okunarak** doğrulandı
(`executable`, 0.95):

| Crate | Lisans | | Crate | Lisans |
|---|---|---|---|---|
| base64 | MIT OR Apache-2.0 | | schemars | MIT |
| bincode | MIT | | notify-debouncer-full | MIT OR Apache-2.0 |
| bytes | MIT | | **syntect** (fork-only) | MIT |
| clap / clap_complete | MIT OR Apache-2.0 | | **image** (fork-only) | MIT OR Apache-2.0 |
| crossterm | MIT | | **trash** (fork-only) | MIT |
| ctrlc | MIT/Apache-2.0 | | **time** (fork-only) | MIT OR Apache-2.0 |
| interprocess | 0BSD OR Apache-2.0 | | serde_ignored (upstream-only) | MIT OR Apache-2.0 |
| libc, png, regex, serde, serde_json, sha2, unicode-width, windows-sys | MIT OR Apache-2.0 | | ratatui, tokio, tracing, tracing-subscriber | MIT |
| toml | MIT OR Apache-2.0 | | portable-pty (vendored) | MIT |

#### Tam bağımlılık ağacı (301 paket) taraması

```
TOTAL PACKAGES IN LOCK: 301
RESOLVED FROM LOCAL CACHE: 299   MISSING: 2  (herdr'ın kendisi + vendored portable-pty=MIT)

--- LİSANS HİSTOGRAMI ---
 142  MIT OR Apache-2.0        83  MIT                17  MIT/Apache-2.0
  14  Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT
   9  Apache-2.0 OR MIT         4  Unlicense OR MIT    3  Apache-2.0/MIT
   … (Zlib, ISC, 0BSD, BSD-2/3-Clause, CC0-1.0, WTFPL, BSL-1.0, Unicode-DFS-2016)

--- COPYLEFT / KISITLAYICI TARAMASI (GPL|MPL|EUPL|CDDL|OSL|EPL|SSPL|BUSL|Elastic) ---
r-efi-5.3.0: MIT OR Apache-2.0 OR LGPL-2.1-or-later   ← üçlü seçenek, MIT/Apache seçilebilir
r-efi-6.0.0: MIT OR Apache-2.0 OR LGPL-2.1-or-later   ← aynı
```

`claim:` **301 paketlik bağımlılık ağacının tamamı permissive lisanslıdır. Copyleft-only
(GPL/AGPL/MPL/SSPL/BUSL) tek bir bağımlılık yoktur.** Tek "hit" olan `r-efi` üçlü lisans sunar ve
MIT/Apache-2.0 seçeneği alınabilir.
`evidence:` executable — Python taraması, `Cargo.lock`'tan 301 paket çıkarıldı, 299'u lokal registry
cache'inde bulundu ve gerçek `Cargo.toml` `license` alanları okundu.
`confidence: 0.92` (cache'teki sürümler lock'takinden minör farklı olabilir; crate lisansları
sürümler arası nadiren değişir)

**Sonuç:** Bağımlılıklar **ne AGPL'de kalmayı ne Apache-2.0'a geçmeyi engeller.** Her iki yön de
serbesttir.

#### Vendored kod

| Bileşen | Lisans | Telif sahibi | Relicense'tan etkilenir mi? |
|---|---|---|---|
| `vendor/libghostty-vt` | **MIT** | Mitchell Hashimoto, Ghostty contributors (2024) | **HAYIR** — MIT hem AGPL hem Apache ile uyumlu |
| `vendor/portable-pty` | **MIT** | Wez Furlong (2018) | **HAYIR** |
| `vendor/libghostty-vt/pkg/afl++` | **MIT** | Loris Cro (2024), zig-afl-kit kökenli | **HAYIR** |
| `src/fm/natsort.rs` (**fork-only**) | **MIT** | sxyazi (2023), yazi projesinden uyarlama | **HAYIR** |

`claim:` Vendored bileşenlerin tümü MIT'tir; MIT permissive olduğu için her iki yöne de (AGPL veya
Apache-2.0) sorunsuz akar. **Ancak MIT, telif bildiriminin korunmasını zorunlu kılar** — mevcut
LICENSE dosyaları ağaçta duruyor, bu yükümlülük şu an karşılanıyor görünüyor.
`evidence:` executable — `head vendor/*/LICENSE*`, `head -30 src/fm/natsort.rs`. `confidence: 0.95`

**Fork-only bulgu:** `src/fm/natsort.rs` upstream'de **yoktur**
(`git cat-file -e upstream/master:src/fm/natsort.rs` → yok). Fork'un kendi eklediği, yazi projesinden
MIT altında uyarlanmış bir dosyadır ve telif başlığı düzgün şekilde korunmuştur. Bu iyi bir uygulamadır
ve fork'un lisans hijyeni açısından olumlu bir işarettir.

### BELİRSİZLİKLER ve HUKUKÇU GEREKTİREN NOKTALAR

| # | Belirsizlik | Neden repo'dan çözülemez | Kim çözer |
|---|---|---|---|
| 1 | Upstream'in 46 dış katkı sahibinden relicense onayı alıp almadığı | Repo'da CLA/DCO yok, onay kaydı yok; süreç repo dışında (mail/Discord) yürümüş olabilir | Upstream'e sorulur / hukukçu |
| 2 | CLA'sız tek taraflı relicense'ın hukuki geçerliliği | Saf hukuk sorusu, teknik kanıtla cevaplanamaz | **Hukukçu — ZORUNLU** |
| 3 | Apache-2.0 → **AGPL**v3 (GPLv3 değil) yönü için tek cümlelik resmî FSF beyanı | FSF license-list ve GPL FAQ bu spesifik çifti tek cümlede ele almıyor (`⚠️ doğrulanamadı`, confidence 0.75) | Hukukçu / FSF licensing@fsf.org |
| 4 | Upstream LICENSE'ında telif satırının boş (`Copyright [yyyy] [name]`) ve NOTICE dosyasının olmaması | Olgu doğrulandı; hukuki sonucu belirsiz | Hukukçu |
| 5 | Fork'un LICENSE/README'sinde duran "ticari lisans: hey@herdr.dev" teklifinin halka açık forkta kalması | CyPack o kodun telif sahibi değil → o teklifi veremez; ancak metin miras alınmış | Hukukçu + housekeeping |
| 6 | "herdr" ismi/markası kullanımı (özellikle hard fork senaryosunda) | Apache §6 marka izni vermiyor; AGPL'de de ayrı marka hükmü yok | **Hukukçu — marka avukatı** |
| 7 | Fork'un halka açık dağıtımı (CyPack/herdr public) nedeniyle AGPL §13 ağ-kullanım yükümlülüğünün tetiklenip tetiklenmediği | Herdr bir terminal uygulaması; "remote network interaction" tanımına girip girmediği yorum gerektirir | Hukukçu |

---

## KULLANICININ KORKUSUNA DOĞRUDAN CEVAP

> *"Sonra kaynak kodunu kapatmak falan isteyecekler mi, desteklerimiz boşa mı gidecek?"*

**Kısa cevap: Hayır. Bugüne kadar yayınlanmış hiçbir kod geri alınamaz. Fork kendi başına devam
edebilir.**

Lisans metinlerine dayanarak, madde madde:

1. **Geri alınamazlık lisansın kendi ifadesidir.** AGPL-3.0 §2: *"All rights granted under this
   License are granted for the term of copyright on the Program, and are **irrevocable**."*
   Apache-2.0 §2/§3: *"perpetual… irrevocable."* Upstream, yayınladığı hiçbir sürümün lisansını
   geriye dönük iptal edemez.

2. **Fork'un elindeki taban AGPL'dir ve öyle kalır.** `46174563`'e (2026-07-11) kadarki tüm upstream
   kodu fork'a AGPL-3.0-or-later altında geldi. Upstream'in 11 gün sonra yaptığı relicense,
   **daha önce dağıtılmış kopyaları etkilemez.**

3. **819 commit'lik CyPack katkısı %100 CyPack telifindedir.**
   `git shortlog -sne 46174563..HEAD` → tek yazar. Upstream'in bu kod üzerinde hiçbir tasarruf
   yetkisi yoktur. Hangi lisansla dağıtılacağına **CyPack karar verir**.

4. **Yayınlanmış hiçbir release Apache-2.0 değildir.** v0.7.5 dahil tüm release'ler AGPL. Relicense
   henüz tag'lenmemiş bir master commit'idir.

5. **Hard fork için lisanstan kaynaklı engel yoktur.** Tam tarihçe (2026 commit) lokal diskte;
   301 bağımlılığın tamamı permissive; vendored kod MIT. Tek dikkat edilecek şey **isim/marka**
   (Apache §6: marka izni verilmez) — bağımsız devam senaryosunda proje adı değiştirilmelidir.

6. **AGPL'de kalmak aktif bir kalkandır.** AGPL kodu Apache-2.0 ağacına **alınamaz**
   (ASF: *"GPLv3 software cannot be included in Apache projects"*). Yani fork AGPL'de kalırsa,
   CyPack'in katkıları proprietary bir türeve süzülemez. Tersi serbesttir: upstream'in Apache kodu
   fork'a alınabilir.

7. **Asıl risk kaynak kapatma değil, dikkatsiz merge'dir.** §G'de gösterildiği gibi, 125 commit'lik
   merge **hiçbir çakışma üretmeden fork'u sessizce Apache-2.0'a çevirir.** Kullanıcının korktuğu
   "koruma kaybı" en muhtemel olarak upstream'in bir hamlesiyle değil, **kendi merge'imizle**
   gerçekleşir. Bu bilinçli bir karar olmalıdır.

---

## Kanıt özeti (doğrulama için)

| Kaynak | Tip | Confidence |
|---|---|---|
| `git show cd5ea1be`, `git log`, `git diff`, `sha256sum`, `git shortlog`, `git tag --contains` | executable (lokal) | 0.98 |
| `~/.cargo/registry/src/*/Cargo.toml` lisans alanları (299/301 paket) | executable (lokal) | 0.92 |
| `LICENSE` / `upstream/master:LICENSE` birebir metin alıntıları | source (lokal dosya) | 0.98 |
| https://www.gnu.org/licenses/license-list.html | official (FSF) | 0.90 |
| https://www.apache.org/licenses/GPL-compatibility.html | official (ASF) | 0.90 |
| https://www.gnu.org/licenses/gpl-faq.html | official (FSF) | 0.90 |
| https://github.com/CyPack/herdr (public/fork durumu) | official (GitHub) | 0.85 |
| Apache-2.0 → AGPLv3 doğrudan uyum beyanı | ⚠️ **doğrulanamadı** — iki official kaynaktan çıkarım | 0.75 |
| Upstream'in repo dışı relicense duyurusu/onayı | ⚠️ **doğrulanamadı** — repo dışı kanal kontrol edilmedi | — |

---

> ⚖️ **TEKRAR: BU HUKUKİ GÖRÜŞ DEĞİLDİR.** Yukarıdaki tüm repo gerçekleri lokal git komutlarıyla
> doğrulanmıştır (confidence 0.92–0.98). Lisans yorumları FSF ve Apache Software Foundation'ın
> **kendi resmî yayınlarından** birebir alıntılanmıştır (confidence 0.90). Ancak lisans metinlerinin
> somut bir olaya uygulanması hukuki analiz gerektirir. Özellikle §H tablosundaki 7 madde için — ve
> merge kararı verilmeden önce — açık kaynak lisanslaması konusunda uzman bir hukukçuya
> danışılması önerilir.
