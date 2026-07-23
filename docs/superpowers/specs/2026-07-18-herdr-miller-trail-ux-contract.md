# Miller Trail UX Kontratı — KANONİK (kullanıcı direktifi, 2026-07-18; mouse-focus güncellemesi 2026-07-23)

Kaynak: kullanıcının kendi referans implementasyonu **circet-miller**
(`/home/ayaz/cc-dashboard-circet-data-platform/web/src/pages/sections/CircetMillerSection.tsx`,
canlı: `http://127.0.0.1:8771/p/circet-miller`) + superfile/yazi ekran kanıtları.
Kullanıcı sözü: **"millers ui ux i böyle çalışmak zorunda"** — bu dosya o zorunluluğun yasasıdır.
Herdr FM'in mevcut sabit `parent/CURRENT/preview` + resident-cache modeli bu kontrata GÖRE YENİDEN
kurulacak (custom-layout/FM programının çekirdeği).

## YASA 1 — Trail (iz) modeli: kolonlar kökten birikir

```
trail: [ {parent:null, selected:null} ]              // açılış: tek kolon (kök)
klasöre primary tık (kolon i) →
    activeCol = i ; cursor = exact(tıklanan)          // strong focus AYNI kolonda
    child-preview(tıklanan) async hazırlanabilir      // child focus ALMAZ
Right/l/Enter (cursor directory iken) →
    trail = trail[0..=i] ; trail[i].selected = cursor
    trail.push({parent: cursor, selected:first})      // yeni kolon SAĞA ve focus oraya
dosyaya primary tık (kolon i) →
    activeCol = i ; cursor = exact(tıklanan)
    detay/önizleme paneli açılır (sağda, resizable)   // kolon EKLENMEZ
```

- Her görünür kolon **GERÇEK ve YÜKLÜ** içeriktir; her satırı tıklanabilir.
- **"(unavailable)" placeholder kolonu ASLA render edilmez.** Yüklenmemiş ata = gösterilmez;
  gösterilen her şey canlıdır. (FIP-D3'ün nihai cevabı budur — lazy-load tartışması kapandı:
  trail zaten kökten indiği için her kolon tanım gereği yüklüdür.)
- Ata kolonunda BAŞKA bir kardeşe tık → strong focus tam o satıra ve o kolona geçer;
  bounded preview eski sağ dalı hazırlanan kardeşle değiştirebilir fakat child focus alamaz.
  Right/`l`/Enter bu hazırlanmış dala explicit geçiştir. Kullanıcı hiçbir zaman görünmez
  bir focus değişimini geri almak için Left'e basmak zorunda kalmaz.
- Seçili öğe her ata kolonunda vurgulu kalır (yol görsel olarak okunur: kova → werkmap → bölüm → dosya).

## YASA 2 — Odak ve kaydırma

- Explicit Right/`l`/Enter ile odaklanan child kolon **otomatik görünüre kaydırılır** (yatay
  scroll, smooth). Primary click ise tıklanan owner kolonu görünür/aktif tutar; resident child
  yalnız hazırlanmış veri olabilir. Ata kolonları sola taşsa da CANLI kalır.
- Aktif kolon kavramı tek otoritedir (`activeCol`); klavye ve fare aynı seçimi paylaşır
  (FIP-5'te picker için kurduğumuz ilkeyle aynı).
- Primary pointer intent = focus/select; hierarchy crossing = Right/`l`/Enter. Async preview,
  resident depth veya painted branch marker kendi başına `activeCol` değiştiremez.

## YASA 3 — Dosya = detay + evrensel önizleme (sağ panel)

- Dosyaya tık kolon açmaz; sağda **resizable side-panel** açar (overlay/modal DEĞİL — kardeş
  listesi görünür kalmalı; referans yorum: interaction-design-decision-matrix §1).
- Panel içeriği: ad, tür/kind, meta; **evrensel önizleme: foto/pdf/text/xlsx** (referans:
  `CircetFilePreview`). Önizlenemeyen tür açıkça söylenir, sessiz boşluk bırakılmaz.
- FIP-D4 (YENİ SAHA KUSURU): herdr önizlemesi Ghostty'de "(Kitty graphics req.)" basıp foto
  GÖSTERMİYOR — Ghostty kitty-graphics'i destekler; kusur ya host-yetenek algılamada ya da
  server-side render'ın graphics passthrough'unda. Ayrı iz sürülecek.

## YASA 4 — Kolon ergonomisi

- Kolon genişliği **per-index** resizable (Finder gibi; yalnız sürüklenen kolon değişir),
  kalıcı (persist); 160-480 px guard deseni referansta.
- Resize şeridi HER ZAMAN görünür ve tıklanması kolay (kalın, hover vurgulu).
- Satır: ikon + ad (truncate, title-tooltip) + klasörse chevron `›`; silinmiş/gone öğe
  **soluk** ama listede (gizlenmez).
- Çoklu-seçim satır başı checkbox ile; klasör seçimi içeriğiyle (recursive) anlaşılır etiketli.

## YASA 5 — Sol sidebar = doğrudan navigasyon

- FAVORITES/LOCATIONS (superfile: Home/Downloads/…/Pinned/Disks) öğeleri tek tıkla trail'i
  o köke KURAR (deep-link deseni: ancestor zinciri resolve → trail inşa → hedef flash;
  referans: `?focus=` R9 akışı). FIP-D1'in kabul kriteri budur — sadece "çalışsın" değil,
  trail'i doğru kurarak çalışsın.

## Herdr'a uyarlama notları (runtime/client boundary korunur)

- Trail state = client-local sunum durumu (AppState); dizin snapshot'ları mevcut
  `read_directory_snapshot`/watcher altyapısını kullanır — her trail kolonu bir snapshot.
- Bounded'lık korunur: derinlik sınırı (mevcut chain≤32) trail uzunluğu sınırı olarak kalır;
  görünür pencere yatay scroll'dur, "(unavailable)" değil.
- Test noktaları (plan aşamasında zorunlu tablo): directory-primary-click-owner-focus;
  click-sonrası-Up/Down-aynı-kolon; Right-first-child-highlight; async-preview-focus-çalmaz;
  stale-click-inert; trail-truncate-rebranch; auto-scroll-right; dosya-tık-kolon-eklemez;
  unavailable-asla-render; sidebar-favorite-trail-kurar; per-index-resize-persist;
  VIS-07+ Chromium baseline'ları.

*Bu kontrat B-zinciri (custom layout/FM programı) design spec'inin girdi yasasıdır; ona aykırı
hiçbir Miller tasarımı onaylanamaz.*
