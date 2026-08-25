# Headless loop cadence — %98-core spin olayı ve davranış sözleşmesi

*2026-08-25 · commit ailesi: `f384cbc5` (housekeeping kapısı) + `341ade2f` (prof marker +
sayaçlar) + `e54c66b3` (due-geçmiş deadline clamp) · davranışlar: TP-SRV-HK-01/02/03 +
TP-SRV-PROF-01 (`behaviors/shared-surfaces.md`) · kardeş: [resource-doctrine](resource-doctrine.md)
RD1/RD2 · ölçüm günlüğü: `.local/prd/headless-housekeeping-cadence.md` (untracked, laptop)*

Bu doküman iki şey içindir: (1) olayın **davranış dilinde** kalıcı kaydı — bir sonraki kişi
teknik literatüre girmeden ne olduğunu anlasın; (2) döngünün artık hangi **sözleşmeyle**
çalıştığı — yeni feature yazan herkes için bağlayıcı.

---

## Olay — canlıda ölçülen (2026-08-25, 27 pane + 15 claude agent)

| | önce | sonra |
|---|---|---|
| herdr server'ın yediği çekirdek | **1 tam çekirdek (%98)** | **%3,5** |
| Boşuna dönüş | saniyede **167.000** kez | saniyede ~120 kez |
| İşin kendisi (çizim, agent çıktısı işleme) | 13/s | 15/s — **hiç değişmedi** |

## Hata pratikte neredeydi? (teknik literatürsüz)

herdr'ın kalbi tek bir döngü: **"uyu → bir şey olunca uyan → işi yap → tekrar uyu."**

Uyanma sebeplerinden biri de "randevular" (örn. *saat göstergesini 1 sn sonra tazele*,
*oturumu 15 sn sonra kaydet*). Hata şuydu:

> **Bir randevu geçmişte kalmıştı ve onu karşılayan kimse yoktu.** Döngü "randevu zamanı
> geçmiş, hemen uyan!" diyordu, uyanıyordu, randevuyu karşılayacak adım o turda çalışmıyordu,
> randevu geçmişte kalmaya devam ediyordu → tekrar "hemen uyan!" → saniyede 167 bin kez
> **boş uyanış**. Kapı zili takılı kalmış gibi: zil çalıyor, kapıda kimse yok, zil çalmaya
> devam ediyor.

Sinsi tarafı: her uyanışta yapılan iş mikroskobikti (~5 µs), log'a hiçbir şey düşmüyordu —
dışarıdan "sessiz ama %98 CPU" görünüyordu. İkincil kök: her uyanış, randevusu olsun olmasın
**tüm ev-işlerini** (display defter değişimi, dosya işçisi yoklamaları…) baştan yapıyordu;
yani ev-işleri maliyeti agent çıktı hızıyla **çarpılıyordu**.

## Davranış matrisi — önce / sonra (bağlayıcı sözleşme)

| durum | ESKİ davranış | YENİ davranış |
|---|---|---|
| Randevu geçmişte kaldı, karşılayan yok | "HEMEN uyan!" → sonsuz boş dönüş | "En erken 10 ms sonra, ev-işleri turunda bakılır" → dönüş biter (TP-SRV-HK-03) |
| 15 agent aynı anda çıktı basıyor | **Her** çıktı parçasında tüm ev-işleri baştan | Ev-işleri en fazla ~100/s; çıktının kendisi yine anında işlenir (TP-SRV-HK-01) |
| Kullanıcı tıkladı / yazdı | anında | anında — girdi/çıktı/render dalları kapıya girmez; en kötü +10 ms (16 ms render aralığının altı) |
| Ev-işleri turu atlandı ama iş birikmiş olabilir | — | ≤10 ms içinde garanti geri-uyanış (TP-SRV-HK-02, starvation yasağı) |
| Kimse bakmıyor, her şey sessiz | uyur | uyur — **ekstra kalp atışı eklenmedi**; kapı yalnız skip turunda uyanış katlar (RD2) |

## Tekrarlar mı?

**Bu hata: hayır.** Üç kilit:

1. **Yapısal:** clamp, döngünün **tek** uyuma noktasında. Gelecekte kim yeni bir "randevu"
   eklerse eklesin, geçmişte kalan her randevu otomatik 10 ms kuralına çarpar — tek tek
   hatırlamak gerekmez. (`HEADLESS_HOUSEKEEPING_MIN_INTERVAL`, `clamp_due_deadline`,
   `src/server/headless.rs`)
2. **Kayıtlı:** üç davranış isimli testlerle registry'de (TP-SRV-HK-01/02/03) — testin
   sahiplenmediği davranışı bir sonraki upstream merge sessizce silebilir; defter bunu engeller.
3. **Gözlemlenebilir:** aynı sınıftan yeni bir "takılı zil" çıkarsa artık kör değiliz:
   `touch ~/.config/herdr/render-prof.on` → canlı sunucu bir sonraki live-handoff'ta
   profiler'lı doğar (env handoff'tan geçemez — marker tam bunun için var; TP-SRV-PROF-01).
   Log'da saniyelik `loop.tick`, `housekeeping.pass`, `headless.turn avg_us`, `ev.*` sayaçları.
   Bu olayın teşhisi araçsız saatler sürdü; artık dakikalar.

## Çok yoğun kullanım: 10 client + 15 aktif agent

Ölçülmüş taban ([resource-doctrine §4](resource-doctrine.md)) + bu olayın katkısı:

| yük bileşeni | ölçülmüş davranış | 10 client / 15 agent'ta |
|---|---|---|
| Çizim, çok client | toplam iş **plato** (~1 çekirdek, ~64 kare/s toplam); kare client'lara paylaşılır (1→24 fps, 10→~6 fps/client) | toplam ~sabit; kimse sistemi katlayamaz |
| Ağ | 0,7→1,4→**~2,8 MB/s tavan** (N=1→2→4+); bayt ∝ değişen hücre | tavanlı |
| Agent çıktısı işleme | 20 üretken pane tabanı %13 — gerçek iş | ~%10-15 bandı |
| Boştaki pane'ler | 0,036 %/pane; Unwatched+Exited → dormant | ~bedava |
| **Ev-işleri turu** (bu olay) | ESKİ: çıktı hızıyla çarpılır → YENİ: **≤100/s, agent sayısından bağımsız** | 15 de olsa 50 de olsa aynı |

**Bilinen açık ölçüm:** 10 client × ev-işleri kombinasyonu — her pass client başına küçük bir
display defter-değişimi yapar; 10 client'ta ~1000 küçük swap/s. Kâğıt üstünde rahat, canlıda
ölçülmedi → resource-doctrine §8 gap satırı + görev: HP kutusunda 10-sahte-client + çıktı seli
stres lab'ı, sayılar §4'e işlenecek.

## Süreç dersleri (yöntem)

- **ptrace/eu-stack örnekleme profili iki kez yanlış suçlu gösterdi** — ptrace-stop
  syscall-restart sınırına düşer; örneklem sistematik biaslıdır. Suçlu atfı in-process
  sayaçla yapılır (bu yüzden marker + sayaçlar kalıcı ürün oldu).
- İzole lab CPU'su canlıyı temsil etmez (küçük surface seti) ama **oran sayaçları**
  (tick/pass) eder — lab, mekanizma kanıtı için; CPU kanıtı canlı teslimden.
- İlk dalga (housekeeping kapısı) tur maliyetini 300µs→5µs indirdi ama canlı CPU'yu
  düşürmedi — "düzeldi" ancak kullanıcının katmanında ölçülünce söylenir; sayaçlar gelmeden
  kapatılsaydı yanlış zaferdi.
