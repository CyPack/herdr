# F1 · P2 — `pane.audio.stream` sunucu ucu: PRD + kod-grafiği bulguları + test noktaları

**Tarih:** 2026-08-29 · **Dal:** `feat/media-audio-api` (worktree `~/projects/herdr-worktrees/media-audio-api`, taban `master@a970a0fb`) · **Oturum:** HERDR WEB BROWSER (birleşik kanal: TUI-browser-51 ⊕ Media-transport-53)
**Önkoşul:** F0 ✅ · F1 motor ✅ · P1 istemci ✅ (hepsi `a970a0fb`'de ve canlıda: server pid 1773354, 11:33)
**Kanonik:** `docs/references/remote-media-transport.md` §L2/§L5/§L6 · `docs/patterns/remote-media-transport.md` RM3·RM4·RM6·RM9·RM12 · ⛔RA3·RA4·RA7·RA8
**Sözleşme:** `~/.claude/handoffs/herdr/Media-transport-53-handoff.md` §6 (peer'in bağımsız kaydı: `TUI-browser-51-handoff.md` §12.6)
**Anlık PRD (bu belgenin tohumu):** `Media-transport-53-prd.md` — C1-C7, 5 sessiz halka, 10 TN. Bu belge onu kod grafiğiyle **tamamlar**.

> Bu alan beş kez araştırıldı; bu belge araştırma değil, **uygulama planıdır**. Kanonik belgenin D1-D7
> yanlışları (`2026-08-28-f1-audio-research.md` §4) burada tekrar edilmez, kabul edilmiş sayılır.

---

## 1. Amaç

Bir pane'e **ses beslemek** (dış üretici → API) ve o sesin `media.audio.sink=[opus]` anlaşmış her uzak
istemcinin hoparlöründen çıkması. Bugün: motor (`AudioStream`, `src/media/stream.rs`) ve istemci
(`PlaybackThread` + `client/mod.rs::media_open`) hazır; aralarında **hiçbir çağıran yok**
(`grep -rn "AudioStream::" src/ | grep -v stream.rs` → boş, 2026-08-29 11:3x ölçümü).

## 2. Kapsam

**İÇİNDE:** L-A şema · L-B sunucu okuyucusu · L-C app yaşam döngüsü + istemci fan-out · L-D kredi bağlama ·
L-E test + davranış kaydı + doküman.
**DIŞINDA (gerekçeli):** kaynak yakalama (P3 — protokol-dışı besleyici, ayrı iş) · sürüklenme (P4) · video (F2) ·
QUIC (F3, ertelendi) · çerçeve parçalama (HOL, F2/F3) · PWA (F4).

## 3. KOD GRAFİĞİ BULGULARI (codebase-memory-mcp, fast reindex 32 881 düğüm / 157 690 kenar, 2026-08-29 11:3x)

| Soru | Araç | Bulgu | P2'ye etkisi |
|---|---|---|---|
| Kardeş uç nerede, kimi çağırıyor | `search_graph serve_frames` | `api.server.pane_graphics_stream.serve_frames` in=2 (serve_with_timeouts, test) out=30; `dispatch_stream_open` → `dispatch_to_app` (`ApiRequestMessage{stream_active}`) | Okuyucu **aynı üç dikişi** kullanır: `dispatch_stream_open` (ack öncesi, aktiflik Arc'ı taşır) · `dispatch_to_app_with_timeout` (kare) · `write_json_line*` |
| App tarafı açılış kimden geliyor | `trace_path handle_pane_graphics_stream_open` | çağıran yalnız `src/app/api.rs` dispatch (`Method::PaneGraphicsStreamOpen` kolu, :1238) + 5 test | Ses için üç yeni kol: `PaneAudioStreamOpen/Chunk/Close` |
| Aktiflik Arc'ı slota nasıl bağlanıyor | grep (`attach_pane_graphics_stream_active`) | iki yol: `headless.rs:4063` (server) + `app/runtime.rs:89` (gömülü) — ikisi de yanıt `ok` ise `Runtime::attach_stream_active` | Ses için **iki yola da** `attach_pane_audio_stream_active` |
| Sahipsiz akış süpürücüsü | `headless/pane_graphics.rs:23-35` | `cancel_inactive_pane_graphics_streams(|owner| slot canlı mı)` her tick (`headless.rs:756`) | Ses için ayrı registry + `cancel_inactive_pane_audio_streams` aynı tick'te |
| İstemciler nerede, fan-out nasıl | `headless.rs:3299 self.clients.insert` · `clients.rs:31 ClientConnection{writer: Option<ClientWriter>, capabilities: CapabilitySet}` · `render_targets` | İstemci ve yazıcı **HeadlessServer**'da, App'te değil | Fan-out headless katmanında; App yalnız **outbound olay** üretir |
| Kredi nerede düşüyor | `client_transport.rs:1278` | `ClientMessage::MediaCredit{..} => continue` ("no stream can be open yet") | `ServerEvent::ClientMediaCredit{client_id,stream_id,chunks}` eklenir; `ServerEvent` tek exhaustive match (`headless.rs:3238`) + handoff süzgeci (`:3587`) |
| Medya şeridi | `client_transport.rs:385 try_send_media` (kapasite 64, Full döner) · `try_pop` :434 medya **en son** | `TP-MEDIA-PRIO-01` zaten çivili | Chunk → `writer.media.try_send`; Full → düşür |
| Kontrol şeridi (güvenilir) | `frame_server_message` (`headless.rs:2865`) + `writer.control.send` (8 kullanım) | MediaOpen/Close bu yoldan | Sıra garantisi: control her zaman media'dan önce boşalır → Open, ilk Chunk'tan önce ulaşır |
| İstemci ne bekliyor | `client/mod.rs:2360 media_open` | `negotiated_value(AUDIO_SINK)==codec` değilse **credit 0** döner; kabulde `PlaybackCommand::Open{stream_id,target_latency_us}` + ilk `TimeSync`; `media_tick` her 100 ms `MediaCredit{stream_id, chunks}` (kredi = `CREDIT_ROOM(32) − tutulan`) | Sunucu kredi seviyesini **istemciden** alır; iade/biriktirme YOK |
| ⚠ İndeks kapsamı | `index_repository` çıktısı `excluded: src/media` | medya sembolleri grafikte YOK (dışlanmış dizin) | `src/media/*` için grep son adım — indeksin bayatlığı değil, kapsamı |

## 4. Katman bölümlemesi + SAHİPLİK (client-side / server-side / araç zinciri)

```
P2
├── L-A  şema (server, upstream dosya)   src/api/schema/panes.rs + schema.rs + api/mod.rs + api/server.rs::api_method_name
│        Method::PaneAudioStream(PaneAudioStreamParams)       #[serde(rename="pane.audio.stream")] #[schemars(skip)]
│        Method::PaneAudioStreamOpen/Chunk/Close              #[serde(skip)] iç kollar
│        PaneAudioStreamParams{pane_id, sample_rate_hz=48000, channels=2, format="f32le", owner(skip)}
│        PaneAudioChunkParams{pane_id, owner, pcm: Vec<u8>} · PaneAudioStreamCloseParams{pane_id, owner, failed, detail}
├── L-B  okuyucu (server, fork dosyası)   src/api/server/pane_audio_stream.rs
│        şekil doğrulama ACK ÖNCESİ → owner=next_owner() → dispatch_stream_open → registry → ack →
│        döngü: read_exact(7680) → dispatch Chunk → yanıt ok değilse kapat · EOF kare sınırında=Ended · yarım kare=Failed
├── L-C  app yaşam döngüsü (fork dosyası)  src/app/pane_audio.rs + src/app/api/pane_audio.rs
│        Runtime{sessions: pane→Session{owner, stream_id≥1, AudioStream, active, client_credit, sayaçlar}, next_stream_id, outbound}
│        open → Outbound::Open · chunk → offer(pcm, now_us()) → Send → Outbound::Chunk · close → Outbound::Close
├── L-C' fan-out (server, headless)       src/server/headless/pane_audio.rs
│        abone = full app client ∧ writer ∧ negotiated_value(AUDIO_SINK)==opus
│        Open/Close → control şeridi · Chunk → media.try_send (client kredisi>0) · Full → dropped_full
├── L-D  kredi (server, upstream dosya)    client_transport.rs (MediaCredit → ServerEvent) + headless.rs (event → set_client_credit)
└── L-E  test + kayıt + doküman           pane_audio_stream.rs testleri · pane_audio.rs testleri · headless testleri ·
         behaviors/shared-surfaces.md TP-MEDIA-API-01/02/03 · CREDIT-02 · CAP-07 · LANE-02 · PTS-01 · docs/next socket-api.mdx (en+ja+zh-cn)
```

**İstemci tarafı (P1, hazır, DOKUNULMAZ):** `client/mod.rs` Open/Chunk/Close/TimeSyncReply · `PlaybackThread` · `AudioSink` (cpal macOS / ffplay Linux).

### 4.1 Çok-istemci kredi modeli (S49 kararı)

`AudioStream.credit` tek sayı; canlıda 3+ App-client bağlı (S48 ölçümü) ve `ffplay` kurulu olduğu için
yerel istemciler de `AUDIO_SINK=[opus]` ilan ediyor. Karar: **encode bir kez, kredi istemci-başına.**
`Session.client_credit: HashMap<client_id,u16>`; her kareden önce `stream.set_credit(max(client_credit))`
(en az bir istemcinin yeri varsa kodla), `Send` sonrası yalnız kredisi >0 olan istemcilere `try_send`, başarılıysa o
istemcinin kredisi −1. `Full` → `dropped_full`, kredi **iade edilmez** (kredi istemcinin seviyesidir, 100 ms'de bir
yeniden ilan edilir — peer notu 4). Hiç abone yoksa `offer` `DroppedNoCredit` üretir, zaman/seq ilerler (TP-MEDIA-SOURCE-01).

## 5. Bağımlılık zinciri + SESSİZ başarısızlık avı

```
L-A ─► L-B ─► L-C ─► L-C' ─► L-D ─► L-E
```

| # | Adım atlanırsa / ters yapılırsa | Belirti | Sınıf | Kapatan TN |
|---|---|---|---|---|
| S1 | L-C' abone süzgeci (`negotiated_value`) yok | sink'siz istemciye MediaOpen+Chunk; sunucu sağlıklı, ses yok, istemci `credit 0` ile cevap verir ama sunucu chunk üretmeye devam eder | ⚠ SESSİZ | TP-MEDIA-CAP-07 |
| S2 | L-D kredi bağlanmaz | `credit` 0 kalır → `offer` hep `DroppedNoCredit` → API `ok`, ses yok | ⚠ SESSİZ | TP-MEDIA-CREDIT-02 |
| S3 | `AudioStream::new(.., start_pts_us=0)` | istemci her kareyi süresi geçmiş sayar (`pts+target < now`) → hepsi düşer | ⚠ SESSİZ | TP-MEDIA-PTS-01 |
| S4 | `try_send` Full'u sayılmaz | şerit büyümez ama kaynak neden kaybettiğini bilmez; teşhis imkânsız | ⚠ SESSİZ | TP-MEDIA-LANE-02 |
| S5 | yarım kare kabul (pad/truncate) | ses 20 ms parçalar yerine kayar, drift; derlenir, çalar, yanlış | ⚠ SESSİZ | TP-MEDIA-API-01 (7679/7681) |
| S6 | şekil kontrolü ack SONRASI | üretici ack alır, veri yollar, sonra red — kaynak yarım kalır | 🔊 gürültülü ama sıralama yanlış | TP-MEDIA-API-01 (shape) |
| S7 | MediaOpen media şeridinden | ilk Chunk Open'ı geçebilir (media şeridi FIFO ama control ayrı) | ⚠ SESSİZ (yarış) | TP-MEDIA-LANE-02 (Open control şeridinde) |
| S8 | pane ölünce session kalır | reader süresiz bekler, `pw-record` zombisi (P3-3) | ⚠ SESSİZ | TP-MEDIA-API-03 (retain_live_panes) |
| S9 | kapanışta MediaClose yok | istemci sink'i açık tutar, `media_tick` kredi göndermeye devam eder | 🔊 (stats) | TP-MEDIA-API-03 |
| S10 | `ServerEvent` yeni varyantı handoff süzgecine takılır | handoff sırasında kredi düşer → o an ses susar, sonra düzelir | kabul (tasarım) | — |

## 6. TEST NOKTALARI — ne · beklenen · NEDEN (icradan ÖNCE, hepsi önce KIRMIZI)

| TP | Ne | Beklenen | NEDEN |
|---|---|---|---|
| **TP-MEDIA-API-01a** | JSON istek + ack + 3×7680 bayt gövde | app'e `PaneAudioStreamOpen` (owner `pane.audio.stream:`) + 3 `PaneAudioStreamChunk` (pcm.len()=7680) + EOF'ta `PaneAudioStreamClose{failed:false}` | `pane_graphics_stream_dispatches_binary_frames`'in kardeşi; sayı yanlışsa ses hızlanır/yavaşlar |
| **TP-MEDIA-API-01b** | 7679 bayt gövde | 0 Chunk, `invalid_frame` hata satırı, Close{failed:true, detail "short frame"} | pad/truncate sürüklenme kaynağı (S5) |
| **TP-MEDIA-API-01c** | 7681 bayt gövde | 1 Chunk, sonra `invalid_frame` + Close{failed:true} | kare sınırı kuralı iki yönde |
| **TP-MEDIA-API-01d** | `format:"s16le"` / `sample_rate_hz:44100` / `channels:1` | ack ÖNCESİ `invalid_params`, app'e HİÇ dispatch yok | yanlış şekil kabul edilirse decoder sessizce gürültü çalar (S6) |
| **TP-MEDIA-API-01e** | açılış hatası (app `pane_not_found`) | hata ack olarak iletilir, Close dispatch edilir, ack YOK | graphics `reports_open_errors_before_ack` kardeşi |
| **TP-MEDIA-API-01f** | açılış timeout | `server_unavailable` + Close | graphics kardeşi |
| **TP-MEDIA-API-01g** | ack öncesi kopuş | Close dispatch edilir | graphics kardeşi |
| **TP-MEDIA-API-01h** | `cancel_inactive_pane_audio_streams` owner'ı pasif sayar | okuyucu döner, Close dispatch | pane ölünce akış durur (S8) |
| **TP-MEDIA-API-01i** | damlayan gövde (5 ms'de 1 bayt) | mutlak deadline'da TimedOut | graphics `trickled_..._obeys_absolute_deadline` kardeşi |
| **TP-MEDIA-API-02** | protokol + şema + okuyucu kaynağı | `pipewire|alsa|coreaudio|cpal|pulse` geçmez (case-insensitive) | §L6: platform adı protokole sızarsa macOS/Windows'ta anlamsız mesaj + her yeni altyapı protokol değişikliği |
| **TP-MEDIA-API-03a** | app: open → session | `stream_id==1` (ilk), owner kayıtlı, `Outbound::Open{codec opus, params Audio{48000,2}, target_latency 100_000}` | yaşam döngüsü tohumu; `stream_id 0` istemcide "yok" |
| **TP-MEDIA-API-03b** | app: chunk (kredi 5) | `Outbound::Chunk{seq 0, pts=açılış pts}` ve `sent==1` | uçtan uca üretim |
| **TP-MEDIA-API-03c** | app: yanlış owner chunk | `stream_conflict`, chunk yok | graphics `stream_owner_controls_only_its_named_layer` kardeşi |
| **TP-MEDIA-API-03d** | app: close | `Outbound::Close{Ended}`, session silinir; ikinci close no-op | idempotan kapanış |
| **TP-MEDIA-API-03e** | app: pane silinir → `retain_live_panes` | session düşer + `Outbound::Close{Failed,"pane closed"}` + active=false | S8 |
| **TP-MEDIA-API-03f** | app: ikinci open aynı pane, ilk aktifken | `stream_conflict`; ilk pasifse (active=false) yenisi alır (stale reclaim) | graphics stale-slot deseni |
| **TP-MEDIA-CREDIT-02a** | headless: `ClientMediaCredit{c1, s1, 0}` → chunk | `DroppedNoCredit`, hiçbir istemciye `try_send` yok | S2 |
| **TP-MEDIA-CREDIT-02b** | `{c1,s1,5}` → 6 chunk | 5 `Send`+gönderim, 6. `DroppedNoCredit`; istemci kredisi 0 | seviye semantiği |
| **TP-MEDIA-CREDIT-02c** | iki istemci: c1 kredi 0, c2 kredi 3 → chunk | encode 1 kez, yalnız c2'ye gönderim | çok-istemci modeli (§4.1) |
| **TP-MEDIA-CREDIT-02d** | c2 kopar (`ClientDisconnected`) | kredi kaydı düşer; sonraki chunk `DroppedNoCredit` | ölü istemci kredisi hayalet göndermesin |
| **TP-MEDIA-CAP-07** | istemci `AUDIO_SINK` anlaşmamış (boş capabilities) | `MediaOpen` gönderilmez, `declined_no_sink==1`; opus'lu ikinci istemciye gönderilir | S1 |
| **TP-MEDIA-LANE-02** | `test_channel_through_queue` ile: Open, 64 chunk + 1 | Open **Control** şeridinde; 64 Media; 65. `Full` → `dropped_full==1`; drain sırası Control önce | RA4 + B4 + S7 |
| **TP-MEDIA-PTS-01** | open anında `now_us()` ölç → ilk Chunk pts | `open_now ≤ pts ≤ open_now + 1 s`, ve 0 DEĞİL | S3 (istemci expiry kuralı) |
| **TP-MEDIA-OPUS-INTEROP** (P6, bu dalda değil) | encoder çıktısı → `ffmpeg -c:a libopus` | çözülür, süre eşit | B9 |

### 6.1 Faz-test tablosu (S47 eki: katman · cross-check çifti · phase-test)

| Faz | Katman | Cross-check çifti (iki bağımsız kanıt) | Phase-test |
|---|---|---|---|
| P2.2 şema | server/API | derleme (`cargo check`) ⟷ `api_method_name` exhaustive match | `herdr-api.schema.json` **değişmedi** (`#[schemars(skip)]`) |
| P2.3-4 okuyucu | server/API | birim (dispatch sayısı) ⟷ ham soket ile bayt-düzeyi test (`local_stream_pair`) | TP-MEDIA-API-01a..i |
| P2.5-8 app+fanout+kredi | server/app + headless | app birim (`Outbound`) ⟷ headless birim (`test_channel_through_queue` şerit etiketi) | TP-MEDIA-API-03, CREDIT-02, CAP-07, LANE-02, PTS-01 |
| P2.9 platform sızıntısı | protokol | grep testi ⟷ `wire.rs` `MediaParams` kaynak okuması | TP-MEDIA-API-02 |
| P2.11 canlı | ürün | `PlaybackStats.played>0/underruns==0` ⟷ **kulak** (kullanıcı) | C1 |
| P2.12 kapı | tümü | `just check` HP ⟷ `behavior_registry_check` | C6 |

## 7. Kabul kriterleri (C1-C7, `Media-transport-53-prd.md` §3'ten, değişmedi)

C1 sentetik ton duyulur · C2 yarım kare ret · C3 sink'siz istemciye akış yok · C4 kredi bağlı · C5 kontrol gecikmesi
değişmez (TP-MEDIA-PRIO-01 mevcut) · C6 `just check` yeşil + registry OK · C7 protokolde platform adı yok.

## 8. Görevler (defter: `.local/TASKS-30.md` `## S49`)

P2.0-P2.10 ✅ (S49; master `8a25e169`: 24 TN → 32 test + PLAYBACK-08) · P2.11 C1 ✅ lab (S50 13:30:
izole client + ffplay, `played=417 underruns=12` 10 s; kök neden TP-MEDIA-DISPATCH-01 — ana döngü
grafik-inaktifken audio isteğini fan-out'suz yola sokuyordu, fix `48189f48`) · P2.12 iniş-1 ✅ 13:08
(`8a25e169`), iniş-2 (dispatch fix) sürüyor · KALAN: P2.13 mutasyon probu · Mac kulak kanıtı (kullanıcı) ·
P4/P8 uzun ölçüm (underruns=0 hedefi bu ölçümde kapanır).

## 9. Riskler

| # | Risk | Azaltma |
|---|---|---|
| R1 | `api/server.rs`, `schema.rs`, `headless.rs`, `client_transport.rs` upstream dosyaları — senkron çakışması | tek satırlık kollar; TP kayıtları `shared-surfaces.md`; `herdr-fork-discipline` |
| R2 | `ServerEvent` yeni varyantı — exhaustive match (`headless.rs:3238`) + handoff süzgeci | derleyici + TP-MEDIA-CREDIT-02 |
| R3 | `pane_graphics_stream.rs` içindeki `read_exact`/`ReadTimeouts` private | `pub(super)` görünürlüğü (upstream'e küçük diff, davranış değişmez) |
| R4 | HP kutusu tek ağaç; iniş kuyruğu (gfx dalı önce) | el sıkışma + `flock` (var) + dar filtre |
| R5 | `docs_translation_parity` kapısı | yalnız başlık sayar → ja/zh-cn tabloya hücre, başlık eklenmez |
| R6 | çok-istemci yerel `ffplay`: 3 App-client → 3 ffplay aynı sesi çalar | P1 tasarımı; P2 kapsamı dışı, deftere not (kullanıcı kararı: yerel istemci sink ilanı) |

## 10. V

```
V(P2) = TN 24 (API-01a..i 9 + API-02 1 + API-03a..f 6 + CREDIT-02a..d 4 + CAP-07 1 + LANE-02 1 + PTS-01 1 + INTEROP 1)
      + kriter 7 (C1..C7) = 31 → DUR: V=0 · iki tur sabit · eskalasyon (§D)
```

*2026-08-29 · feat/media-audio-api · HERDR WEB BROWSER*
