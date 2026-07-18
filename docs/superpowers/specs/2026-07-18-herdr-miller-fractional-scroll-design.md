# Herdr Miller Trail Kesirli Yatay Kaydırma Tasarımı

Tarih: 2026-07-18

Durum: Kullanıcı tarafından yönü onaylandı; yazılı spesifikasyon incelemede

Kapsam: Native Files Miller Trail yatay viewport'u

Girdi kontratı:
`docs/superpowers/specs/2026-07-18-herdr-miller-trail-ux-contract.md`

## 1. Bağlam ve kök neden

Canlı Trail entegrasyonu yatay kaydırmayı destekliyor; ancak viewport otoritesi
`first_visible: usize` ile yalnız kolon indeksini tutuyor. Bir wheel olayı hedef indeksi
`-1` veya `+1` değiştiriyor. Geometri de yalnız tam kolonlardan başladığı için her olay bir
kolon genişliği kadar sıçrıyor ve kısmen görünür kolon üretilemiyor.

Bu davranış işlevsel fakat kullanıcının kanonik Circet Miller referansındaki akıcı yatay
gezinti hissini karşılamıyor. Kullanıcı önerilen çözümü onayladı:

- bir wheel olayı yaklaşık görünür kolon genişliğinin `1/3`'ü kadar ilerler;
- viewport hücre düzeyinde hareket eder;
- solda ve sağda kısmi kolonlar görünebilir;
- trackpad'in tekrarlanan wheel olayları doğal bir akış üretir;
- zamana bağlı animasyon, easing veya yeni bir timer/event-loop mekanizması eklenmez.

## 2. Hedefler

1. Her yatay wheel olayında tam kolon yerine yaklaşık `1/3` kolon ilerlemek.
2. Farklı genişlikteki kolonlarda adımı ilgili kolonun kendi genişliğinden türetmek.
3. Kısmi ilk/son kolonları doğru kırpılmış içerik ve doğru fare hedefleriyle göstermek.
4. Trail navigasyonundaki mevcut aktif kolonu otomatik görünür tutma davranışını korumak.
5. Manuel kaydırma, resize, stale frame ve tek-kolon durumlarında deterministik kalmak.
6. Render saflığını ve client-local sunum sınırını korumak.
7. Test akışını throwaway ortam temizliğiyle başlatıp aynı temizlikle kapatmak.

## 3. Kapsam dışı

- Zaman tabanlı smooth-scroll animasyonu, easing eğrisi veya momentum fiziği.
- Sunucu protokolü, socket mesajı, runtime state ya da persisted session şeması değişikliği.
- Dikey satır kaydırma davranışının değiştirilmesi.
- Kolon resize UX'inin yeniden tasarlanması.
- Stable Herdr server/socket veya açık kullanıcı terminal oturumlarına dokunmak.
- FIP-6.3 mouse harness araştırması; bu çalışma ürünün mevcut mouse event yolunu kullanır.

## 4. Değerlendirilen yaklaşımlar

### A. Tam zaman tabanlı animasyon

Hedef ofsete easing ile birkaç frame içinde ulaşır. Görsel olarak en yumuşak seçenek olsa da
TUI event-loop'una timer, ara frame ve iptal/birleştirme durumu ekler. Hızlı ardışık wheel,
resize ve branch değişimlerinde ek state makinesi gerektirir. Bu kapsam için gereksiz risklidir.

### B. Sabit hücre adımı

Her event örneğin 8 hücre ilerler. Basit ve deterministiktir; ancak 16 hücrelik ve 60 hücrelik
kolonlarda aynı hareket hissini vermez. Per-index kolon genişliği kontratıyla zayıf uyumludur.

### C. Kolon genişliğinin üçte biri kadar hücre adımı — ONAYLI

Viewport mutlak içerik hücresi ofseti tutar. Her event, viewport'un ilerleme yönündeki referans
kolon genişliğinin yaklaşık üçte biri kadar hareket eder. Minimum adım bir hücredir. Ratatui'nin
hücre tabanına doğal uyar, kısmi kolonları mümkün kılar ve timer olmadan trackpad'de akıcı
hissedilir.

## 5. Durum modeli ve otorite

Yatay viewport client-local sunum durumudur. Trail seçimi veya runtime/session gerçeği değildir.

Manuel viewport iki kavramsal alan taşır:

- `offset_cells`: Trail içerik başlangıcından viewport'un sol kenarına kadar mutlak hücre
  ofseti.
- `follow_active`: Trail navigasyonunun aktif kolonu otomatik görünür tutup tutmadığı.

Eski `first_visible` indeks otoritesi üretim kararından çıkarılır. Geriye dönük test veya
projeksiyon ihtiyacı varsa ilk görünür kolon, `offset_cells` ve kolon aralıklarından türetilir;
ikinci bir mutable otorite olarak tutulmaz.

`offset_cells` yalnız viewport'u değiştirir. Şunlar aynı kalır:

- `TrailState::active_col`;
- seçili exact path;
- cursor ve bulk-selection otoriteleri;
- operation intent;
- Files generation ve revision.

Bu değişiklik yalnız TUI/client katmanındadır; server, protocol ve private client socket'e alan
eklenmez.

## 6. Mantıksal yatay koordinat sistemi

Her kolon ve divider Trail içerik uzayında mutlak bir yarı-açık aralık taşır:

```text
column_i = [column_start, column_end)
divider_i = [divider_start, divider_end)
viewport = [offset_cells, offset_cells + stage.width)
```

Toplam içerik genişliği kolon genişlikleriyle divider genişliklerinin doygun toplamıdır.

```text
max_offset = total_content_width.saturating_sub(stage.width)
offset_cells = offset_cells.clamp(0, max_offset)
```

Terminal daralırsa veya kolon genişlikleri değişirse manuel ofset yeni `max_offset` değerine
yeniden clamp edilir. İçerik stage'den darsa `max_offset = 0` olur ve horizontal wheel
durumu değiştirmez.

## 7. Kesirli adım semantiği

Bir wheel event'i bir yönde tek deterministik adım üretir:

```text
step_cells = max(1, ceil(reference_column_width / 3))
```

Referans kolon yön-duyarlıdır:

- sağa giderken `offset_cells` hücresini içeren veya onun sağındaki ilk kolondur;
- sola giderken `offset_cells - 1` hücresini içeren veya onun solundaki ilk kolondur;
- ofset bir divider içindeyse ilerleme yönündeki en yakın kolon kullanılır;
- kolon bulunamazsa sınırdaki ilk/son kolon kullanılır.

Bu seçim, farklı genişlikte kolonlarda yeni kolona geçildiği anda o kolonun kendi ergonomik
adımını kullanır. `ceil` küçük kolonların üç event'te ilerleyebilmesini ve sıfır adım
oluşmamasını sağlar. Son adım sınırı aşarsa sonuç `0..=max_offset` aralığına clamp edilir.

Mevcut input eşlemeleri korunur:

- `ScrollLeft` veya `Shift+ScrollUp`: sola bir kesirli adım;
- `ScrollRight` veya `Shift+ScrollDown`: sağa bir kesirli adım.

Wheel delta büyüklüğü platformlar arasında kararlı normalize edilmediği için tek event tek adım
sayılır. Trackpad akıcılığı ardışık event'lerden gelir; üretim kodu wall-clock veya frame
zamanına bağlı olmaz.

## 8. Auto-follow ve manuel kaydırma

Trail'in YASA 2 davranışı korunur:

- klasör aktivasyonu, rebranch, deep-link veya klavye ile aktif kolon değişimi
  `follow_active = true` yapar;
- compute aşaması aktif kolonun tamamını mümkünse görünür kılan en küçük ofseti seçer;
- aktif kolon stage'den genişse kolonun başlangıcı hizalanır;
- kullanıcı yatay wheel kullandığında `follow_active = false` olur;
- manuel modda resize yalnız ofseti clamp eder, viewport'u aktif kolona geri sıçratmaz;
- follow modunda resize aktif kolon için görünürlüğü yeniden hesaplar.

Manuel olarak ancestor kolonlarına bakmak Trail seçimini değiştirmez. Sonraki gerçek navigasyon
olayı follow modunu yeniden etkinleştirir.

## 9. Kısmi kolon geometri ve render kontratı

Bir kolonun görünür kısmı mantıksal kolon aralığıyla viewport aralığının kesişimidir:

```text
visible = logical_interval ∩ viewport
source_x = visible.start - logical_interval.start
dest_x = stage.x + visible.start - viewport.start
visible_width = visible.end - visible.start
```

Üretim geometri kodu negatif `u16` koordinat üretmez. Önce geniş bir tamsayı uzayında aralık
hesabı yapılır, yalnız görünür kesişim Ratatui `Rect` alanına güvenli biçimde çevrilir.

Her görünür kolon projeksiyonu şunları taşır:

- değişmeyen mantıksal Trail kolon kimliği;
- kolon içindeki yatay kaynak ofseti;
- stage içindeki kırpılmış hedef rect;
- kırpılmış satır/action rect'leri;
- varsa kırpılmış divider rect'i.

Render yalnız kırpılmış hedefe yazar. Sol taraftan taşan bir kolon için doğrudan stage dışında
bir Ratatui rect çizilmez. Uygulama, tam mantıksal kolon satırını geçici bir buffer'a çizip
görünür hücre dilimini kopyalayabilir veya eşdeğer saf bir clip projeksiyonu kullanabilir;
hangi yöntem seçilirse seçilsin geometri ve input aynı `source_x/dest_rect` snapshot'ını
tüketir.

Divider, başlık, chevron, checkbox ve action hedefleri kendi görünür kesişimleri boş değilse
yayımlanır. `Rect::intersection(...)` sonucu `.is_empty()` ile değerlendirilir; sıfır-alanlı
nonzero-origin rect görünür hedef sayılmaz.

## 10. Fare, kimlik ve fail-closed davranışı

- Fare hit-test yalnız stage ile kesişmiş görünür rect'lerde çalışır.
- Kırpılmış/görünmeyen hücreler tıklanabilir alan üretmez.
- Görünür satırın mantıksal `(trail_index, entry_index, exact_path)` kimliği korunur.
- Generation/revision uyuşmazlığı mevcut sözleşmedeki gibi event'i inert bırakır.
- Ofset veya kolon genişliği bayatsa input kendi geometrisini yeniden icat etmez.
- Wheel olayı selection, operation, cursor veya filesystem mutation üretmez.
- Resize sırasında boşalan bir hit rect stale bir komuta dönüşmez.

## 11. Test noktaları

| ID | Neden | Beklenen sonuç | Kanıt |
|---|---|---|---|
| TP-TRAIL-FSCROLL-01 | Tam-kolon sıçramasının geri gelmesini önlemek | Eşit genişlikli kolonlarda ilk iki sağ wheel sonrası ilk kolon kısmen görünür; yaklaşık üçüncü event bir nominal kolon ilerlemesine ulaşır | Rust viewport/adım testi |
| TP-TRAIL-FSCROLL-02 | Per-index genişlikler adım hissini değiştirmeli | Dar ve geniş kolon sınırlarında referans kolon yön-duyarlı seçilir; adım `ceil(width/3)`, minimum 1 hücredir | Rust mixed-width testi |
| TP-TRAIL-FSCROLL-03 | Taşma ve boşluk önlenmeli | Sol sınır 0, sağ sınır `max_offset`; tek kolon/stage'den dar içerik wheel'de inerttir | Rust boundary testi |
| TP-TRAIL-FSCROLL-04 | Auto-follow ile manuel scroll çatışmamalı | Wheel follow'u kapatır; manuel resize clamp eder ama sıçratmaz; navigasyon follow'u açıp aktif kolonu görünür yapar | App state/invariant testi |
| TP-TRAIL-FSCROLL-05 | Eski frame yeni Files instance'ını hareket ettiremez | Stale generation/revision wheel girdisi ofset, selection ve intent'i değiştirmez | Adversarial identity testi |
| TP-TRAIL-FSCROLL-06 | Kısmi kolon görünmez tıklama alanı üretmemeli | Tüm row/action/divider rect'leri stage içinde veya boş; ayrık rect overlap kontrolü `.is_empty()` kullanır | Saf geometri/hit testi |
| TP-TRAIL-FSCROLL-07 | Render saflığı ve tek geometri otoritesi korunmalı | Aynı state byte-identical buffer üretir; render state değiştirmez ve filesystem I/O yapmaz | Buffer mutation/purity testi |
| TP-TRAIL-FSCROLL-08 | Değişiklik Trail navigasyon davranışını bozmamalı | Rebranch, deep-link ve keyboard navigation aktif kolona auto-follow eder; one-column bounded testi ve 10k invariant ailesi yeşil kalır | Mevcut regression aileleri |
| VIS-12 | Gerçek Ratatui hücrelerinde kısmi kolon görünümünü kanıtlamak | Chromium baseline'ında sol kolon kısmen kırpılmış, orta kolon canlı ve sağdaki sonraki içerik görünür; stage dışı çizim yoktur | Spec-scoped Playwright Chromium snapshot |

### RED kanıt sırası

1. Hücre ofseti ve `1/3` adım beklentisini tanımlayan testler mevcut indeks-bazlı modelde fail.
2. Kısmi kolon geometri testi mevcut tam-kolon geometrisinde fail.
3. Input/auto-follow/adversarial testleri yeni state seam'i olmadığı için fail.
4. VIS-12 fixture'ı kısmi kolon üretemediği için fail veya beklenen snapshot'tan ayrılır.

Test zaten geçerse mevcut davranışı karakterize ediyor demektir; RED testi yeni sözleşmeyi
kanıtlayacak biçimde düzeltilmeden üretim koduna geçilmez.

## 12. Uygulama katmanları ve atomik commit'ler

1. `test: require fractional miller trail scrolling`
   - yalnız Rust RED testleri ve gerekiyorsa VIS-12 fixture/spec;
   - `cargo nextest ... --no-fail-fast` ile gerçek RED kanıtı.
2. `fix: add fractional miller trail scrolling`
   - hücre ofseti, yön-duyarlı `1/3` adım, clipping geometri, input ve auto-follow;
   - minimum GREEN kodu, üretimde `unwrap()` yok.
3. Gerekirse ayrı `test: approve fractional miller scroll visual baseline`
   - yalnız yeni VIS-12 baseline;
   - spec-scoped `--update-snapshots`;
   - önce ham buffer/snapshot mutation kanıtı.
4. `docs: record fractional miller scroll closure`
   - task registry, handoff, gate kanıtları ve yayın SHA'ları.

RED ve GREEN aynı commit'e konmaz. Her commit öncesi `cargo fmt --check` ayrı komutta çalışır
ve exit code doğrulanır. Yalnız hedef dosyalar stage edilir; `.superpowers/` asla stage edilmez.

## 13. Doğrulama ve temizlik kapıları

Kullanıcının talebi gereği canlı/manuel doğrulama daima temizlikle başlar ve temizlikle biter.
Kanonik tek giriş `.local/herdr-trail-test.sh` olur:

1. başlangıçta yalnız testin sahibi olduğu throwaway XDG/socket/server kalıntısını semantik
   `server stop` yoluyla kapatır ve test dizinini temizler;
2. tüm `HERDR_*` değişkenlerini unset ederek izole debug server açar;
3. test/manuel doğrulama komutunu yürütür;
4. success, failure veya signal çıkışında aynı test-sahipli server'ı semantik olarak durdurur;
5. stable Herdr socket, açık terminal, tmux veya kullanıcı process'ine dokunmaz; `kill/pkill`
   kullanmaz.

Katman kapanışında aşağıdaki kapılar çalışır:

- focused fractional-scroll Rust testleri;
- Trail/Miller/File Manager ilgili Rust aileleri;
- `cargo nextest run --locked --no-fail-fast`;
- `cargo fmt --check` ayrı komut;
- Linux clippy `--all-targets --locked -- -D warnings`;
- Windows clippy, `LIBGHOSTTY_VT_SIMD=false`;
- Playwright Chromium tam suite; yeni baseline güncellemesi yalnız VIS-12 spec scope'unda;
- maintenance Python ve Bun suite'leri;
- `git diff --check`;
- production `unwrap()` ve legacy first-visible otorite taraması;
- codebase-memory tek-worker reindex ve yeni viewport sembolü snippet doğrulaması.

Her Cargo komutundan önce `PATH="$HOME/.local/bin:$PATH"` sağlanır. nextest RED kanıtı
`--no-fail-fast` kullanır.

## 14. Kabul kriterleri

Çalışma ancak şu koşullar birlikte sağlanınca tamamdır:

1. Bir wheel olayı tam kolona sıçramaz; ilgili kolon genişliğinin yaklaşık üçte biri ilerler.
2. Sol ve sağ kısmi kolonlar doğru hücre kırpmasıyla render edilir.
3. Görünmez hücreler fare/action hedefi üretmez.
4. Manuel scroll resize'da korunur ve clamp edilir; navigasyon auto-follow'u geri açar.
5. Stale frame ve tek-kolon durumları fail-closed/inert kalır.
6. VIS-12 dahil Playwright Chromium suite'i yeşildir.
7. Full Rust, iki clippy, maintenance, Bun, fmt ve source audit kapıları yeşildir.
8. İzole manuel test temizlikle başlayıp temizlikle kapanır; stable Herdr'a dokunulmaz.
9. CyPack `feat/native-fm` ve `master` aynı FF SHA'ya yayınlanır; upstream'a hiçbir işlem
   yapılmaz.

## 15. Rollback ve risk sınırı

Değişiklik client-local viewport state'inde sınırlıdır. RED commit güvenli test checkpoint'i,
GREEN commit işlevsel checkpoint'tir. Yayın öncesi geri alma commit bazında yapılabilir;
yayınlandıktan sonra forward-fix uygulanır.

En büyük risk görsel kırpma ile hit-test'in ayrışmasıdır. Bunu tek immutable geometri
snapshot'ı ve TP-TRAIL-FSCROLL-06 kapısı sınırlar. İkinci risk manual scroll'un navigasyon
auto-follow'u kalıcı bozmasıdır; TP-TRAIL-FSCROLL-04 ve mevcut 10k invariant ailesi bunu
korur. Timer/animation eklenmemesi event-loop ve render determinism riskini kapsam dışında
tutar.
