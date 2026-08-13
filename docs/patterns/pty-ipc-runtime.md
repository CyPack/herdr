---
doc: herdr-patterns-pty-ipc-runtime
domain: pty-ipc-runtime
created: 2026-07-25
status: canonical — her pattern kaynak-etiketli + confidence taşır
id_space: PI1–PI19 (pattern) · PA1–PA11 (anti-pattern)
registry: docs/references/pty-ipc-runtime.md
map: .cartography/pty-ipc-runtime-SYSTEM-MAP.json
agentic_triggers:
  - "pty spawn · pty resize · pty eof · pty kapanmıyor · reader takılıyor · zombie child"
  - "ipc protokol · socket path · framing · handshake · protocol version · capability"
  - "daemon client ayrımı · detach attach · session persist · reattach resync"
  - "conpty · windows named pipe · platform farkı"
---

# PTY & IPC Runtime — Pattern Kataloğu

> **Nasıl okunur.** Her pattern `[PIxx] ad` + **ne** + **KULLAN** + **KULLANMA**
> (over-engineering guard) + `kaynak-etiketi:satır` + confidence taşır. Kaynak etiketleri
> `docs/references/pty-ipc-runtime.md` registry'sine çözülür; hepsi
> `~/.cartography/refpool/` altında **klonlanmış canlı koda** işaret eder.
>
> **Bağımsızlık kuralı.** `[zynk]` herdr fork'udur → bir pattern'in doğrulamasında
> **bağımsız ikinci kaynak sayılmaz**. Registry'deki uyarıya bak.

---

## A. PTY YAŞAM DÖNGÜSÜ

### `[PI1]` slave-drop-after-spawn · **EOF'un ön koşulu** · conf 0.95

**Ne.** `openpty()` bir master/slave çifti verir. Child spawn edildikten *hemen sonra*
`drop(pair.slave)` çağrılmalıdır. Slave fd açık kaldığı sürece kernel PTY'yi "hâlâ yazan
biri var" sayar; child ölse bile master'dan **EOF gelmez** ve reader döngüsü sonsuza kadar
bloke olur.

```rust
let child = pair.slave.spawn_command(cmd)?;
drop(pair.slave);              // ← bu satır olmadan reader ASILI KALIR
let reader = pair.master.try_clone_reader()?;
```

**KULLAN.** `portable-pty` ile her PTY açılışında — istisnasız.
**KULLANMA.** Kasten kalıcı slave tutuyorsan (child yeniden başlatılacak, PTY korunacak) —
ama o zaman EOF'u ayrı bir sinyalle (child exit watcher) üretmek ZORUNDA'sın.

*Kaynak:* `[orbt]` `crates/orbt/src/daemon/pty.rs:53` · `[portable-pty-src]`
`examples/bash.rs:26`, `examples/narrow.rs`, `examples/whoami_async.rs` (upstream örneklerinin
**tamamı** bu satırı içerir) · `[zynk]` test suite'inde 20+ yerde (derivative — teyit değil, sinyal)

---

### `[PI2]` blocking-io-off-runtime · **PTY I/O async değildir** · conf 0.95

**Ne.** `portable-pty`'nin reader/writer'ı **senkron `std::io`**'dur. Bunları doğrudan bir
`tokio::spawn` task'ında çalıştırmak runtime worker thread'ini bloke eder. Doğru yapı:
okuma/yazma `spawn_blocking` içinde döner, async dünyaya `mpsc`/`broadcast` ile köprülenir.

```rust
tokio::task::spawn_blocking(move || {          // reader
    loop { match reader.read(&mut buf) {
        Ok(0) => break,                        // EOF (bkz. PI1)
        Ok(n) => { let _ = event_bus.send(ServerEvent::PaneOutput{ .. }); }
        Err(e) => { warn!("PTY read error: {e}"); break; }
    }}
});
tokio::task::spawn_blocking(move || {          // writer
    while let Some(data) = input_rx.blocking_recv() { writer.write_all(&data)?; writer.flush()?; }
});
```

Dikkat: blocking bağlamda `send().await` YOK → `blocking_send` / `blocking_recv`.

**KULLAN.** tokio runtime'ı olan her PTY sahibi (daemon, server).
**KULLANMA.** Runtime yoksa (saf senkron TUI) — `std::thread::spawn` yeterli, tokio ekleme.

*Kaynak:* `[orbt]` `daemon/pty.rs:73` (reader), `:108` (writer) · `[running-process]`
`crates/running-process/src/broker/backend_sdk/frame_client.rs:15` bu tuzağı yorumda açıkça uyarıyor:
*"`spawn_blocking` will block the runtime worker thread"* · `[sshx]`
`crates/sshx/src/terminal/windows.rs:56` ConPTY spawn'ı da `spawn_blocking`'e alıyor

---

### `[PI3]` master-shared-for-resize · **master'ı canlı tut, Sync yap** · conf 0.9

**Ne.** Master handle sadece okuma için değil — `resize()` ve fd'nin açık kalması için de
gerekir. Reader klonlandıktan sonra master **düşürülmemeli**; paylaşımlı ve `Sync` olmalı ki
dış state (`RwLock<SessionRuntime>`) içinde yaşayabilsin.

İki kanıtlı varyant:
- `Arc<Mutex<Box<dyn MasterPty + Send>>>` — `[orbt]` `daemon/pty.rs:12,66`
- `parking_lot::Mutex<Option<Box<dyn MasterPty + Send>>>` — `[oly]` `session/pty.rs:28`
  Yorumu aynen: *"Wrapped in a `parking_lot::Mutex` so that `PtyHandle` is `Sync`, which lets
  the outer `SessionRuntime` live behind a `parking_lot::RwLock`."*
  `Option` olması **kasıtlı**: kapanışta `take()` ile fd serbest bırakılır (bkz. PI5).

**KULLAN.** Resize edilebilir / uzun ömürlü her pane.
**KULLANMA.** Tek-atımlık komut çalıştırma (spawn → oku → bitir); orada master'ı lokal tut.

---

### `[PI4]` output-batch-window · **coalesce, yoksa event fırtınası** · conf 0.8

**Ne.** PTY read loop'u her `read()` için event yayarsa `cat huge_file` saniyede yüzlerce
event üretir. Okumalar küçük bir zaman penceresinde (`~8ms`) biriktirilip tek `PaneOutput`
olarak yayılır.

**KULLAN.** Yüksek hacimli çıktı beklenen her pane; render frame'ine bağlı sistemler.
**KULLANMA.** Düşük gecikme kritikse ve çıktı zaten seyrekse — pencere gereksiz gecikme katar.

*Kaynak:* `[orbt]` `CLAUDE.md §9.6` (design_doc — *"batches reads within an 8 ms window before
emitting a `PaneOutput` event"*). ⚠️ **Tek kaynak, `open` statüsünde** — herdr'da uygulanmadan
önce ikinci bağımsız kanıt veya lokal ölçüm gerekir.

---

### `[PI5]` nonblocking-reap-then-release · **kill ≠ temizlik** · conf 0.9

**Ne.** Child ölümü üç ayrı adımdır ve karıştırılmamalıdır:
1. **gözlem** — `try_wait()` bloke etmeyen exit kontrolü (poll döngüsünde)
2. **zorlama** — `kill()` (yalnız gerektiğinde)
3. **serbest bırakma** — master fd `take()` + writer kanalını *kapalı* bir kanalla değiştir

Üçüncü adım kritik: writer'ı sadece drop etmek yerine kapalı kanalla değiştirmek, sonraki
yazmaların **hızlıca hata vermesini** sağlar (sessizce birikmez):

```rust
pub fn release_resources(&mut self) {
    self.pty_master.lock().take();                 // fd serbest
    let (closed_tx, closed_rx) = mpsc::channel(1);
    drop(closed_rx);                               // alıcı yok → kanal kapalı
    let previous_tx = std::mem::replace(&mut self.writer_tx, closed_tx);
    drop(previous_tx);
}
```

**KULLAN.** Uzun ömürlü session store'u olan her daemon.
**KULLANMA.** Process kapanışında (`main` dönüyor) — OS zaten toplar; ekstra kod ölü ağırlık.

*Kaynak:* `[oly]` `session/pty.rs:70-98` (`release_resources` / `kill` / `try_wait`),
`session/runtime.rs:230`, `session/store.rs:192,911` · `[tenex]` `mux/server/session.rs:407,480`

---

### `[PI6]` detached-capability-responder · **client yokken kim cevap verecek?** · conf 0.85

**Ne.** Detach modunda (daemon çalışıyor, bağlı terminal yok) child uygulama terminal
yetenek sorgusu gönderirse (`CSI 6n` cursor position, `OSC 10/11` renk) cevap veren kimse
olmadığı için **süresiz bloke olabilir**. Daemon bunlara sentetik cevap üretmelidir — ama
**seçici olarak**.

Cevaplanan: `CPR` · `DSR` · `OSC 10/11` (renk).
**Cevaplanmayan** (kasıtlı): `DA1` · `DA2` · `XTVERSION` · `DECRPM` · kitty-keyboard.
`[oly]`'nin gerekçesi aynen: *"These should only be answered by a real terminal or interactive
client. Answering them in detached mode can cause interference with user input and corrupt the
output stream."*

**KULLAN.** Detach/attach destekleyen her PTY daemon'ı — herdr dahil.
**KULLANMA.** Client her zaman bağlıysa; gerçek terminal daha doğru cevap verir.

*Kaynak:* `[oly]` `session/pty.rs:522-594` (`extract_query_responses_no_client`) +
`:540-561` gerekçe yorumu · benzer ama daha dar bir refleks `[orbt]` `daemon/pty.rs:81-92`
(sadece DA1'e `\x1b[?62;4c` ile cevap veriyor — iki proje **cevap kümesinde ayrışıyor**,
bu bir tasarım kararı olduğunun kanıtı)

---

### `[PI7]` cross-chunk-escape-buffer · **ConPTY escape'i ikiye böler** · conf 0.9

**Ne.** Escape dizileri `read()` sınırında bölünebilir; Windows ConPTY bunu **herhangi bir
byte'ta** yapar (hatta baştaki `ESC`'i tamamen düşürebilir — `[35;1R` gibi "bare" formlar).
Filtre stateful olmalı: tamamlanmamış kuyruk `pending` olarak bir sonraki chunk'a taşınır.

```rust
pub struct EscapeFilter { pending: Vec<u8> }   // per-instance tek state; regex'ler static
```

**KULLAN.** PTY çıktısında escape filtreleme/tarama yapan her yer; Windows destekliyorsan ZORUNLU.
**KULLANMA.** Byte'ları hiç yorumlamadan aynen aktarıyorsan (saf passthrough).

*Kaynak:* `[oly]` `session/pty.rs:467-494` (`EscapeFilter`), `:619-747`
(`filter_cpr_chunk_bytes` — bare/split varyant açıklamaları), `:751-782` APC/DCS kısmi
dizi tespiti. Test kanıtı: `test_filter_cpr_chunk_strips_split_dsr_query`,
`test_escape_filter_preserves_invalid_utf8_bytes_across_split_query` (:1334)

---

### `[PI8]` writer-backpressure-typed · **kuyruk doluysa ne olacak?** · conf 0.85

**Ne.** Girdi yazımı `try_send` ile yapılır ve iki hata **ayrı** ele alınır:
`Full` (tüketici yavaş — geçici) vs `Closed` (session öldü — kalıcı). Tek bir `Result`'a
katlamak teşhisi imkânsız kılar.

**KULLAN.** Girdi kaybının sessizce olmaması gereken her yerde.
**KULLANMA.** Sınırsız kanal kullanıyorsan — ama o zaman OOM riskini kabul etmiş olursun.

*Kaynak:* `[oly]` `session/pty.rs:33-42` · kanal boyu örneği `[orbt]` `daemon/pty.rs:31`
(`mpsc::channel::<Vec<u8>>(64)`)

---

### `[PI9]` pty-env-contract · **child'a kimliğini söyle** · conf 0.85

**Ne.** PTY child'ına spawn anında sabit env seti verilir: `TERM=xterm-256color`,
`COLORTERM=truecolor` ve **pane kimliği** (`ORBT_PANE_ID` / herdr'da karşılığı). Pane
kimliği env'i, agent tespitinin process'i kendi pane'ine bağlamasını sağlar.

**KULLAN.** Agent/pane ilişkisi kurulacak her multiplexer.
**KULLANMA.** Kullanıcının `TERM`'ünü ezmek istemiyorsan — o zaman inherit et, ama o zaman
render varsayımların kırılabilir.

*Kaynak:* `[orbt]` `daemon/pty.rs:45-47` + `CLAUDE.md §10.2` (*"`ORBT_PANE_ID` — Set by orbitd
on every PTY child; agent detection reads this"*)

---

### `[PI10]` pty-backend-trait · **taşınabilirliği tek yüzeye hapset** · conf 0.85

**Ne.** Platform farkı (`unix` openpty · Windows ConPTY · `portable-pty` passthrough) tek bir
trait arkasına alınır; üst katman tek imza görür:

```rust
trait PtyBackend {
    fn try_clone_reader(&mut self) -> io::Result<Box<dyn Read + Send>>;
    fn resize(&self, size: PtySize) -> io::Result<()>;
}
```

**KULLAN.** İki+ backend'i gerçekten destekliyorsan veya test için mock gerekiyorsa
(`[oly]`'nin `RuntimeChild::Mock` varyantı `#[cfg(test)]` ile tam bunu yapar).
**KULLANMA.** Tek backend varsa — trait yalnız dolaylılık katar (YAGNI).

*Kaynak:* `[running-process]` `crates/running-process/src/pty/backend.rs:38-42` (trait),
`:125,217,360` (üç ayrı impl) · `[oly]` `session/pty.rs:111-153` (`RuntimeChild` enum + Mock)

---

### `[PI11]` raw-openpty-escape-hatch · **portable-pty şart değil** · conf 0.85

**Ne.** `portable-pty` kullanmadan doğrudan `nix::pty::openpty` + `login_tty` +
`ioctl(TIOCSWINSZ)` ile PTY yönetmek geçerli bir alternatiftir; fd'ler tokio'ya
`AsyncFd`/nonblocking olarak verilebilir. Maliyeti: Windows'u kendin çözersin.

**KULLAN.** fd üzerinde tam kontrol gerektiğinde (özel `winsize`, `login_tty` semantiği,
`portable-pty`'nin soyutlamasının engel olduğu durumlar).
**KULLANMA.** Windows desteği hedefteyse — ConPTY'yi elle yazmak `portable-pty`'yi
vendorlamaktan pahalıdır (herdr zaten vendorlayıp patch'liyor).

*Kaynak:* `[sshx]` `crates/sshx/src/terminal/unix.rs:11,55,109-122` (openpty + TIOCGWINSZ +
TIOCSWINSZ, her `unsafe` bloğu SAFETY yorumlu)

---

## B. IPC PROTOKOL

### `[PI12]` length-prefixed-frame · **stream'de mesaj sınırı yoktur** · conf 0.95

**Ne.** Unix socket / named pipe bir **byte stream**'dir; mesaj sınırını sen koyarsın.
Kanonik çerçeve: `u32 LE uzunluk | payload`.

```
┌─────────────────┬─────────────────────────────┐
│ length: u32 LE  │ payload (bincode/protobuf)  │
└─────────────────┴─────────────────────────────┘
```
Okuma: `read_exact(&mut [0u8;4])` → `u32::from_le_bytes` → `read_exact(vec![0;len])`.

**KULLAN.** Her binary IPC. `bincode`, `protobuf`, `rmp` — payload formatından bağımsız.
**KULLANMA.** Satır-tabanlı JSON protokolü seçtiysen (`\n` sınırdır — `[bohay]` böyle yapıyor);
o zaman payload'da ham `\n` olmadığını garanti etmelisin.

*Kaynak (BAĞIMSIZ ÜÇLÜ — verified):* `[zellij]` `zellij-utils/src/ipc.rs:399-419` (sync) ve
`:496-508` (async) · `[orbt]` `crates/orbt-protocol/src/encoding.rs:14-27` ·
`[bohay]` karşı-örnek: newline-delimited JSON (`src/ipc/api.rs:72-80`)

---

### `[PI13]` frame-size-cap · **cap'siz length prefix = uzaktan OOM** · conf 0.9

**Ne.** Uzunluk alanı okunduktan sonra o kadar byte **ayrılır**. Cap yoksa kötü/bozuk bir
`u32` (≤4 GiB) anında bellek patlatır. Encode tarafında da kontrol edilir ki hatalı mesaj
hiç yola çıkmasın.

```rust
pub const MAX_MSG_BYTES: usize = 4 * 1024 * 1024;
if len > MAX_MSG_BYTES { return Err(ProtocolError::MessageTooLarge(len, MAX_MSG_BYTES)); }
```

**KULLAN.** İstisnasız her length-prefixed protokol.
**KULLANMA.** — (bu pattern'in "kullanma" hâli yok; cap'i büyüt, kaldırma.)

*Kaynak:* `[orbt]` `encoding.rs:9,17-19` + `CLAUDE.md §8.2` (*"Reject larger — defends against
OOM"*; tipik `PaneOutput` <4 KB, tam grid snapshot ≈160 KB → 4 MB rahat tavan)

---

### `[PI14]` runtime-dir-socket-path · **`/tmp`'de sabit ad YASAK** · conf 0.9

**Ne.** Socket yolu öncelik sırasıyla çözülür:
`/run/user/<uid>/` → `$XDG_RUNTIME_DIR/` → `$TMPDIR/<app>-<uid>.sock`.
Runtime dizini zaten kullanıcıya özel ve `0700`'dür; `/tmp`'de sabit ad çok-kullanıcılı
makinede çakışır ve symlink saldırısına açıktır. `$TMPDIR`'a düşülüyorsa **uid adı içinde**
olmalıdır.

**KULLAN.** Her local IPC daemon'ı.
**KULLANMA.** Yolu kullanıcı/env açıkça override ettiyse (`ZELLIJ_SOCKET_DIR` gibi) — ona saygı duy.

*Kaynak (BAĞIMSIZ ÇİFT — verified):* `[orbt]` `crates/orbt-protocol/src/socket.rs:8-35` +
`CLAUDE.md §9.4` (*"F3 fix — DO NOT regress to /tmp"*) · `[zellij]`
`zellij-utils/src/consts.rs:284-288` (`ZELLIJ_TMP_DIR = temp_dir()/zellij-<uid>`),
`envs.rs:29` (`ZELLIJ_SOCKET_DIR` override)

---

### `[PI15]` sun_path-length-guard · **socket yolu 104 byte'ta kesilir** · conf 0.95

**Ne.** `sockaddr_un.sun_path` sabit boyutludur: **macOS/BSD 104**, **Linux/Android/Solaris
108** byte. Uzun session adı + derin `XDG_RUNTIME_DIR` bu sınırı aşar ve bind **sessizce
tuhaf** biçimde başarısız olur. Sınır compile-time sabiti olarak tutulup ada uygulanmalıdır.

```rust
#[cfg(target_os = "macos")]      pub const SOCK_MAX_LENGTH: usize = 104;
#[cfg(not(target_os = "macos"))] pub const SOCK_MAX_LENGTH: usize = 108;
```

**KULLAN.** Socket adına **kullanıcı girdisi** (session adı, worktree adı, proje yolu) karışan
her yer — herdr'da session/worktree adları tam bu kategoride.
**KULLANMA.** Ad tamamen sabitse ve kısaysa.

*Kaynak:* `[zellij]` `zellij-utils/src/consts.rs:276-280` (Windows dalı için ayrı 256:`:326`)

---

### `[PI16]` peer-credential-policy · **dosya izni tek başına yetmez** · conf 0.9

**Ne.** Kabul edilen her bağlantıda peer'in kimliği (UID / Windows SID) okunur ve bir
**politikaya** göre kabul/ret edilir. Politika sabit kod değil, tipli bir seçimdir:

```rust
pub enum PeerCredentialPolicy {
    AllowAny,                               // kimliği okunabilen herkes
    OwnerOnly { uid_or_sid: String },       // yalnız bu sahip
}
impl PeerCredentialPolicy {
    pub fn current_user() -> Option<Self> { // unix: geteuid(), windows: SID
        #[cfg(unix)] { Some(Self::owner_only(unsafe { libc::geteuid() }.to_string())) }
        ...
    }
}
```

Üç incelik:
1. **Sıra önemli** — politika `Hello` frame'i **okunmadan ÖNCE** uygulanır
   (*"Peer credential policy applied before reading a Hello frame"*). Kimliği bilinmeyen
   peer'in gönderdiği hiçbir byte parse edilmez.
2. **Platform farkı gerçek** — macOS `LOCAL_PEERCRED` **pid alanı taşımaz**, dolayısıyla
   Linux `SO_PEERCRED`'de mümkün olan pid çapraz-kontrolü orada yapılamaz. Kod bunu yorumla
   kaydediyor; sessizce varsayılmıyor.
3. **Testle çivilenir** — kimlik-bırakma yolu ayrı bir test dosyasına sahip
   (`tests/broker/peer_creds_drop.rs`).

**KULLAN.** Çok-kullanıcılı makinede çalışabilecek her daemon (= herdr).
**KULLANMA.** Kasten paylaşımlı erişim tasarlıyorsan — o zaman `AllowAny` + **gerçek** bir
yetkilendirme katmanı, "kontrol yok" değil.

### Windows tarafı — ad değil **sahip** doğrulanır

Unix'te dosya izni bir savunma katmanıdır; Windows named-pipe'ta **yoktur**. `\\.\pipe\` ad alanı
makine-global ve adlar tahmin edilebilir → başka bir yerel hesap adı **kapıp** (squat) izin-verici
bir DACL ile sahte veri besleyebilir. `[gwm-cli]`'nin çözümü:

- **Sahip, bağlı kernel nesnesinden okunur** (`GetSecurityInfo` + `SE_KERNEL_OBJECT` +
  `OWNER_SECURITY_INFORMATION`, `EqualSid` ile karşılaştırma) → **PID-yeniden-kullanım yarışı yok**.
- **Her API hatası fail-closed** — okunamıyorsa bağlantı reddedilir.
- Asıl bariyer **ad değil DACL**: `D:P` (protected, kalıtım yok) + tek ACE, `GENERIC_ALL` →
  OWNER RIGHTS (`S-1-3-4`). Ad yalnız *kazara* çakışmayı önler.
- Ad, `USERDOMAIN\USERNAME` üzerinden **enjektif** kaçışla namespace'lenir — kayıplı bir katlama
  iki farklı hesabı aynı ada düşürüp ikincisini kendi varsayılanından kilitlerdi.
- Kontrol **client tarafında** yapılır: istemci bağlandığı sunucunun sahibini doğrular.

*Kaynak (kodda doğrulandı — iki bağımsız platform):*
**Unix** `[running-process]` `crates/running-process/src/broker/server/connection.rs:24-56`
(policy enum + `current_user`), `:470` (`stream.peer_creds()?`), `server/control_socket.rs:63-235`
(her giriş noktasına parametre), `server/hello_handler.rs:400` (macOS LOCAL_PEERCRED notu),
`tests/broker/peer_creds_drop.rs` ·
**Windows** `[gwm-cli]` `src/daemon.rs:1428-1460` (`verify_server_owner`), `:1420-1427`
(tehdit modeli yorumu), `:1078-1090` (enjektif ad + `owner_only_descriptor`), `:1047-1070`
(platform paritesi beyanı)

> ⚠️ **DERS — design_doc ≠ kod.** Bu pattern ilk turda `[orbt]` `CLAUDE.md §8.3/§9.4`'e
> dayanıyordu (*"Server checks SO_PEERCRED UID on every accepted connection"*). Kodda
> doğrulama yapılınca **orbt'de `peercred`/`getpeereid`/`ucred` hiç geçmiyor** — tek eşleşme
> `crates/orbt/src/ssh.rs:204` `remote_uid`, o da SSH üzerinden `id -u` çalıştıran tamamen
> ilgisiz bir fonksiyon. orbt'nin belgesi **aspirational**dı. Pattern ancak `running-process`
> kodunda gerçek implementasyon bulununca `verified` oldu.
> **Kural:** `design_doc` tier'ı tek başına asla `verified` yapmaz — koda çözülmeli.

---

### `[PI17]` hello-welcome-handshake · **versiyon uyuşmazlığını ilk mesajda yakala** · conf 0.85

**Ne.** Bağlantı `Hello{client_version, protocol_version, capabilities}` →
`Welcome{server_version, protocol_version, capabilities∩, FullState}` ile açılır.
`protocol_version` eşleşmezse sunucu tipli bir hata (`ProtocolError{code}`) döner ve kapatır —
yarı-anlaşılan mesajlarla devam etmez.

Evrim kuralı: **additive** değişiklik → `Capabilities` bayrağı; **breaking** → `PROTOCOL_VERSION`
bump. herdr'ın kendi karşılığı `src/protocol/wire.rs::PROTOCOL_VERSION` (bkz. AGENTS.md
"Code Conventions" — bump'ı *son yayınlanan tag*'e göre değerlendirir).

**KULLAN.** Client ve server'ın ayrı ayrı güncellenebildiği her sistem (= herdr).
**KULLANMA.** İkisi de tek binary'den, aynı anda deploy ediliyorsa.

**Çift yönlü varyant + eyleme çevrilebilir ret.** `[mprocs]` handshake'i tek yönlü değil:
client `Hello` yollar, **server kendi `Hello`'suyla cevaplar** ve *her iki taraf da* sürüm
eşitliğini bağımsız doğrular. Uyuşmazlıkta server tipli `Bye { code: UNSUPPORTED_PROTOCOL,
message }` gönderip kapatır — ve mesaj kullanıcıya **ne yapacağını** söyler:
*"daemon (X) speaks protocol N, this binary speaks M; restart it with `… server stop && … up`"*.
Sürüm hatasında kullanıcının elinde eylem yoksa hata yarım kalmış demektir.

*Kaynak:* `[orbt]` `CLAUDE.md §8.3-8.4` + `crates/orbt-protocol/src/messages.rs` ·
`[mprocs]` `src/protocol/conn.rs:128-175` (çift yönlü + `Bye` + eylem mesajı) ·
`[herdr-live]` `src/protocol/wire.rs:16,320,611,937-941` (karşılaştırma tabanı)

---

### `[PI18]` protocol-crate-runtime-free · **kontratı runtime'dan ayır** · conf 0.9

**Ne.** Wire tipleri ve domain modeli **tokio'ya bağımlı olmayan** ayrı crate'lerde yaşar:
`orbt-protocol` (mesajlar) ve `orbt-core` (VT/grid) — ikisi de saf senkron. Bu sayede
protokol testleri runtime kurmadan çalışır ve kontrat sızmaz.

Ek kural: kütüphane crate'leri public API'de `anyhow` **kullanmaz** (`thiserror` ile tipli
hata); yalnız binary'ler `anyhow` kullanır — çünkü `anyhow` çağıranın ihtiyaç duyduğu tip
bilgisini siler.

**KULLAN.** Protokolün birden çok istemcisi olacaksa/olabilecekse.
**KULLANMA.** Tek binary, tek tüketici — crate bölmek gereksiz build karmaşası.

*Kaynak:* `[orbt]` `CLAUDE.md §5.2` (*"`orbt-protocol` and `orbt-core` MUST remain tokio-free"*),
`§9.1` (hata katmanlama tablosu) · yapısal kanıt: `orbt/crates/orbt-protocol/Cargo.toml`
(tokio yok)

---

### `[PI19]` stale-socket-liveness-reclaim · **sil, ama önce canlı mı diye bak** · conf 0.9

**Ne.** Daemon çirkin öldüyse socket dosyası kalır ve `bind` "address in use" ile patlar.
Doğru kurtarma **iki adımlıdır**: (1) dosya var mı, (2) **sunucu gerçekten ölü mü**. Sadece
`remove_file` yapmak, canlı bir sunucunun soketini koparır.

```rust
pub fn cleanup_if_stale(&self) -> bool {
    if self.exists() && !self.is_server_alive() { let _ = self.cleanup(); true } else { false }
}
```
`[fresh-editor]` canlılığı ayrı bir **PID dosyasıyla** kanıtlar ve data/control soketlerini
ayrı tutar.

**KULLAN.** Tek-instance daemon'ı olan her uygulama.
**KULLANMA.** Soket abstract namespace'te (Linux `@` prefix) — orada dosya artığı olmaz.

**Canlılık nasıl ölçülür — üç varyant, üç maliyet:**

| Varyant | Kaynak | Güçlü yanı | Zayıf yanı |
|---|---|---|---|
| **connect probe** — gerçekten bağlanmayı dene | `[herdr-live]` `src/ipc.rs:75-109` | Gerçek durumu ölçer; PID yarışı/yeniden-kullanımı yok | Bağlanma denemesinin kendi maliyeti/timeout'u |
| **PID dosyası** | `[fresh-editor]` `server/ipc/mod.rs:115-138` | Ucuz, bağlanmadan | PID yeniden kullanımı ve yarış koşuluna açık |
| **lock dosyası** | `[mprocs]` `src/daemon/socket.rs:9-30` + `daemon/lockfile.rs` | OS lock semantiği süreç ölümünde otomatik serbest | Platformlar arası lock davranışı farkı |

*Kaynak:* yukarıdaki üç kaynak · karşı-örnek `[bohay]` `src/ipc/api.rs:56` kendi yorumunda
eksikliği itiraf ediyor: *"Best-effort stale-socket reclaim (single-instance dev;
**proper detection arrives with the M2 server**)"* → **canlılık testi olmayan sürüm bilinçli bir borç**

---

### `[PI20]` client-spawns-daemon · **daemon yoksa istemci başlatır** · conf 0.85

**Ne.** İstemci bağlanamazsa hata vermek yerine daemon'ı **kendisi başlatır** ve yeniden bağlanır;
otomatik başlatma kapalıysa hata **eylem talimatı** taşır (*"Daemon is not running. Start it with
`… up`"*). Sıra: lock oku → canlı mı → değilse `cleanup_stale` → `spawn_server_daemon` → bağlan.

**KULLAN.** Kullanıcının daemon'ın varlığını bilmek zorunda olmadığı CLI/TUI'lerde (= herdr).
**KULLANMA.** Daemon'ın ne zaman ve hangi ortamla başlayacağı kritikse (systemd/servis yönetimi
altında) — orada çift başlatma yarışı doğar.

*Kaynak:* `[mprocs]` `src/daemon/socket.rs:9-30`

---

## C. ANTI-PATTERN'LER

| # | YAPMA | Neden kırılır | DOĞRU |
|---|---|---|---|
| `PA1` | `spawn_command` sonrası slave'i tutmak | Child ölse de EOF gelmez; reader thread sonsuza kadar asılı, session "canlı" görünür | `PI1` — `drop(pair.slave)` |
| `PA2` | PTY `read()`'i `tokio::spawn` içinde | Runtime worker bloke; başka task'lar aç kalır, tüm daemon takılır | `PI2` — `spawn_blocking` + kanal köprüsü |
| `PA3` | `/tmp/<app>.sock` sabit adı | Çok-kullanıcıda çakışma; symlink/ön-oluşturma saldırısı | `PI14` — runtime dir + uid |
| `PA4` | Uzunluk-prefix'siz binary stream | Mesaj sınırı kaybolur; iki mesaj birleşir veya yarım parse edilir | `PI12` |
| `PA5` | Cap'siz `u32` uzunluk → `vec![0; len]` | Bozuk/kötü niyetli 4 GiB tahsisi = uzaktan OOM | `PI13` |
| `PA6` | `bincode::Encode` derive etmek (bincode 2.x) | serde yolu ile uyuşmaz; runtime decode hatası. Yalnız `Serialize/Deserialize` derive edip `bincode::serde::*` kullan | `[orbt]` `CLAUDE.md §9.2` "the footgun" |
| `PA7` | `interprocess` 1.x örneğini 2.x'e kopyalamak | 2.x'te `connect`/`bind` `&str` almaz → derlenmez. `path.to_fs_name::<GenericFilePath>()?` | `[orbt]` `CLAUDE.md §9.3` |
| `PA8` | `RwLock` write guard'ını `await` boyunca tutmak | Aynı task içinde read-after-write = **self-deadlock** (tokio RwLock reentrant değil) | Guard'ı blok içine al, `await`'ten önce düşür — `[orbt]` `CLAUDE.md §9.9` |
| `PA9` | Canlılık testi yapmadan socket'i silmek | Çalışan daemon'ın soketi koparılır; istemciler sessizce kopar | `PI19` — `exists() && !is_server_alive()` |
| `PA10` | Detached modda **her** capability query'sine cevap | DA1/DA2/XTVERSION cevabı kullanıcı girdisiyle karışır, çıktı akışını bozar | `PI6` — seçici cevap kümesi |
| `PA11` | Sabit 60fps tick ile redraw | Boşta CPU yanar; pil/termal maliyet. Tick yalnız animasyon/aktivite varken | `[orbt]` `CLAUDE.md §6.5` (*"the naive 'always tick at 60fps' pattern is a bug"*) |

---

## D. ÖLÇEK KARAR MATRİSİ

Aynı problem farklı ölçeklerde farklı cevap ister — büyüme sürpriz olmasın:

| Ölçek | PTY sahipliği | IPC | Framing | Uygun pattern seti | Örnek |
|---|---|---|---|---|---|
| **S0** — tek PTY, IPC yok | Uygulama içi, tek thread | — | — | `PI1`, `PI3` | `[tui-term]` (498 node) |
| **S1** — çok PTY, tek process | `HashMap<PaneId, PtyHandle>` | — | — | + `PI2`, `PI5`, `PI8`, `PI9` | `[mprocs]`, `[skim]` |
| **S2** — daemon + tek client | Daemon'da | Unix socket | length-prefix + bincode | + `PI12`–`PI15`, `PI19` | `[oly]`, `[shell-compose]` |
| **S3** — daemon + çok client, detach/attach | Daemon'da | Unix socket + broadcast | + handshake/versiyon | + `PI6`, `PI16`–`PI18`, lossy-broadcast resync | `[orbt]`, **herdr** |
| **S4** — + ağ/SSH, çapraz makine | Daemon'da | socket + ağ transport | protobuf + şifreleme | + oturum anahtarı, akış kontrolü | `[zellij]`, `[sshx]` |

**herdr S3'tedir** ve S4'e (remote/SSH) doğru bakıyor → `[zellij]` ve `[sshx]` bir üst
basamağın referansı olarak havuzda tutuluyor.

**Lossy broadcast uyarısı (S3+).** `tokio::sync::broadcast::send` abone geri kalırsa
**tasarımı gereği veri düşürür**. Kurtarma yolu protokolde olmalı: istemci `Lagged` hatasını
görünce `RequestFullState` gönderir, sunucu tam durumla cevaplar. Bu bir bug değil, kabul
edilmiş bir takas — ama **resync yolu yoksa** bug olur.
*Kaynak:* `[orbt]` `CLAUDE.md §11.2`

---

## E. HERDR'A UYGULAMA NOTU (henüz yapılmadı — sadece harita)

Bu katalog **prior-art**tır; herdr koduna hiçbir değişiklik yapılmadı. Bir pattern'i
almadan önce:

1. **Zaten var mı?** `search_graph(project="home-ayaz-projects-herdr", query="…")` ile
   herdr'ın kendi `src/pty/`, `src/ipc.rs`, `src/protocol/` karşılığını bul.
2. **Bağımsız kanıtı var mı?** `[zynk]` sayılmaz (fork). `open` işaretli `PI4` ve `PI16`
   ikinci kanıt bekliyor.
3. **AGENTS.md sınırı.** Runtime/client boundary guardrail'i gereği: paylaşılan runtime
   gerçeği → server/API; sunum durumu → TUI. Bir pattern bu ayrımı bulanıklaştırıyorsa alma.
4. **Protokol dokunuyorsa** `PROTOCOL_VERSION` kuralını uygula (additive → capability).

---
*v1.0.0 — 2026-07-25 · reference-registry Adım-3 artefaktı · 17 repo, 19 pattern, 11 anti-pattern.*
*Kaynak havuzu: `~/.cartography/refpool/` (klonlu + codebase-memory-mcp indeksli).*
