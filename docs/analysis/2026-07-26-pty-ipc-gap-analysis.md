---
doc: pty-ipc-gap-analysis
domain: pty-ipc-runtime
created: 2026-07-26
repo_head: 8ded0e79
branch: feat/native-fm
status: analiz — herdr koduna DEĞİŞİKLİK YAPILMADI
method: her PI için herdr src/ karşılığı grep + search_graph ile arandı, satır kanıtıyla işaretlendi
related:
  - docs/patterns/pty-ipc-runtime.md
  - docs/references/pty-ipc-runtime.md
  - .cartography/pty-ipc-runtime-SYSTEM-MAP.json
---

# PTY/IPC Pattern Gap Analizi — herdr vs prior-art havuzu

**Skor: 19 pattern → 11 VAR (6'sı havuzdan İLERİDE) · 6 KISMİ · 2 YOK**

> **2026-07-26 güncelleme.** Havuz 17/17 `full` modda yeniden indekslendi ve rol
> taksonomisiyle sınıflandırıldı; 7 `shallow` repo `verified`'a çıktı. Bu tur `PI16` için
> **ikinci bağımsız kaynak** (`gwm-cli`, Windows) ve `PI17`/`PI19` için yeni varyantlar
> buldu, ayrıca **BULGU 2**'yi (Windows named-pipe erişim bariyeri) ortaya çıkardı.

İşaretler: `✅` var · `✅⁺` var **ve havuzdaki en iyi örnekten ileride** · `⚠️` kısmi ·
`❌` yok · `❓` doğrulanamadı

---

## A. PTY yaşam döngüsü

| PI | Durum | herdr karşılığı (kanıt) | Not |
|---|:-:|---|---|
| **PI1** slave-drop-after-spawn | `✅⁺` | `src/pty/backend/unix.rs:36` `drop(pair)` — slave *ve* master orijinali birlikte; master önce `fd::duplicate_cloexec_fd` ile dup'lanıyor (`:30-31`) | **Havuzdan ileride:** hiçbir referans projede olmayan bir test bunu çiviliyor — `unix.rs:73-93` `portable_pty_setup_leaves_one_parent_pty_fd`, `/proc/self/fd` tarayıp parent'ta **tam 1** PTY fd kaldığını doğruluyor. Windows dalı (`backend.rs:32-39`) açık `drop` yerine örtük scope-drop'a güveniyor — ConPTY'de zararsız ama asimetri |
| **PI2** blocking-io-off-runtime | `✅` | `src/pty/actor.rs:141,153,181` dedike `std::thread::spawn` (writer/reader/control); unix'te ayrıca `src/pty/actor/unix.rs` poll + `wake_fd` döngüsü | Farklı varyant: `spawn_blocking` yerine **kalıcı OS thread + kendi event loop'u**. Prensip aynı (blocking I/O runtime dışında), uygulama daha kontrollü |
| **PI3** master-shared-for-resize | `✅⁺` | `src/pty/backend/unix.rs:26-31` master `OwnedFd` olarak dup'lanıp aktöre veriliyor; `actor.rs:186` resize control kanalından | `Arc<Mutex<Box<dyn MasterPty>>>` (orbt/oly) yerine **fd sahipliği** — kilit yok, `Sync` sorunu yok. Daha temiz çözüm |
| **PI4** output-batch-window | `❌` | `src/pty/actor/unix.rs` read loop'unda zaman-pencereli birleştirme bulunamadı | Pattern zaten `open` (tek kaynak). **Ölçmeden uygulama** — önce yüksek hacimli çıktıda event/sn ölç |
| **PI5** nonblocking-reap-then-release | `⚠️` | `wait()` var (`src/pane.rs:1927`, blocking, `spawn_blocking` içinde); `child.kill()`/`wait()` yalnız test yolunda (`unix.rs:90-91`) | **`try_wait()` üretim kodunda yok.** `oly`'nin üç-adımlı ayrımı (gözlem/zorlama/serbest-bırakma) ve özellikle **writer'ı kapalı kanalla değiştirme** (`release_resources`) karşılığı yok — `actor.rs:108` `shutdown()` sadece flag + kanal kapatıyor |
| **PI6** detached-capability-responder | `✅` | `src/pane.rs:2431-2441` `terminal_responses` VT emülatöründen üretilip PTY'ye yazılıyor; `src/pane/cursor.rs:89` `CursorPositionSettleState` | **Mimari olarak farklı çözüm:** herdr'da VT state *her zaman* server'da yaşar, dolayısıyla `oly`'nin ayrı `extract_query_responses_no_client` yoluna ihtiyaç yok. Daha temiz — `no_client`/`detach` özel dalı yok, çünkü gerekmiyor |
| **PI7** cross-chunk-escape-buffer | `✅` | `src/ghostty/mod.rs:72` `Partial` durumu — vendored libghostty-vt içinde | VT parser vendored ve patch'li (`vendor/libghostty-vt.patches.md`) → kısmi dizi durumu upstream'de çözülü |
| **PI8** writer-backpressure-typed | `✅` | `src/pty/actor.rs:76-85` `try_send` + `accepting` flag'i ile `TrySendError::Closed` erken dönüşü | `Full`/`Closed` ayrımı korunuyor; ek olarak quiesce (`initially_quiesced`) durumu var |
| **PI9** pty-env-contract | `✅` | `src/pane.rs:61-62` `TERM`/`COLORTERM`; `:126` `HERDR_PANE_ID_ENV_VAR`, `:129` gerektiğinde `env_remove` | Kaldırma yolu da var (orbt'de yok) |
| **PI10** pty-backend-trait | `⚠️` | `src/pty/backend.rs` + `backend/unix.rs` — `#[cfg]` ile **modül** ayrımı, ortak `SpawnedPty` tipi | Trait yok; iki platform iki ayrı `spawn_with_portable_pty` sunuyor. AGENTS.md "Platform code is isolated" ilkesine uygun → **bilinçli tasarım, gap değil**. Mock/test backend'i yok (oly'nin `RuntimeChild::Mock`'u gibi) |
| **PI11** raw-openpty-escape-hatch | `⚠️` hibrit | `portable-pty` ile açıp `src/pty/fd.rs` üzerinden ham fd'ye iniyor (`OwnedFd`, `duplicate_cloexec_fd`, `drain_wake_fd`) | `sshx`'in tam-ham yolu ile `portable-pty`'nin tam-soyut yolu arasında **kasıtlı orta nokta**: Windows'u portable-pty'ye bırak, Unix'te fd kontrolünü al |

## B. IPC protokol

| PI | Durum | herdr karşılığı (kanıt) | Not |
|---|:-:|---|---|
| **PI12** length-prefixed-frame | `✅⁺` | `src/protocol/wire.rs:783-867` — `u32` LE prefix (`:849`), `read_exact_or_eof` ile **kısmi okuma birleştirme** (`:864-866`) | Havuzda kimse partial-read reassembly'yi bu kadar açık ele almıyor |
| **PI13** frame-size-cap | `✅⁺` | `wire.rs:20` `MAX_FRAME_SIZE = 2 MiB`, `:23` explicit büyük-veri için ayrı cap, `:836-846` `u32::MAX` **truncation** guard'ı, `:19` DoS gerekçesi yorumda | orbt'de cap var ama truncation guard'ı yok; herdr iki ayrı cap seviyesi tanımlamış |
| **PI14** runtime-dir-socket-path | `⚠️` | `src/session.rs:169-171` `config_dir()/sessions/<name>/herdr.sock`; `config/io.rs:30-34` `XDG_CONFIG_HOME` → `$HOME/.config/herdr` | `/tmp` sabit adı **yok** (iyi). Ama `XDG_RUNTIME_DIR` de kullanılmıyor → socket **kalıcı** dizinde yaşıyor: reboot sonrası artık dosya kalır (PI19 bunu çözüyor) ve **yol uzunluğu riski artar** (aşağıdaki bulgu) |
| **PI15** sun_path-length-guard | `✅⁺` *ama kısmi kapsam* | `src/remote/unix.rs:1979,1996` 104/108 açıkça ele alınmış + **hash'li fallback**; `:3040,3047` `local_forward_socket_path_fits_in_sun_path` testi | Havuzun en iyisi (zellij yalnız sabit tanımlıyor, herdr hash fallback + test yapıyor) — **ama yalnız SSH-forward yolunda.** API socket yolunda karşılığı yok → aşağıdaki bulgu |
| **PI16** peer-credential-policy | `❌` **Unix'te azaltılmış / Windows'ta yok** | `peercred\|getpeereid\|ucred\|peer_cred` **hiç geçmiyor**; tek `geteuid` `src/server/clipboard_image.rs:76`, ilgisiz | **Tek net güvenlik gap'i.** Unix'te azaltıcı var: `api/server.rs:83` + `ipc.rs:277` `restrict_socket_permissions` (0600) + `remove_socket_file_if_owned`. **Windows'ta hiçbir şey yok** → aşağıdaki bulgu |
| **PI17** hello-welcome-handshake | `✅` | `wire.rs:16` `PROTOCOL_VERSION = 18`; `:320` `Hello`, `:611` `Welcome`, `:937-941` sürüm karşılaştırma + tipli uyumluluk sonucu ve kullanıcıya yönelik mesaj | Referans uygulamayla eşdeğer; herdr'ın bump kuralı AGENTS.md'de ayrıca disipline edilmiş |
| **PI18** protocol-crate-runtime-free | `⚠️` | `src/protocol/` **modül** olarak ayrı, ama herdr **tek crate** (`Cargo.toml`'da `[workspace]` yok) | Ayrım var, **derleyici tarafından zorlanmıyor**: bugün protokol modülüne tokio import etmeyi hiçbir şey engellemiyor. orbt'de bu ayrı crate ile çivilenmiş. Runtime/client boundary guardrail'i (AGENTS.md) ile aynı yöne bakıyor |
| **PI19** stale-socket-liveness-reclaim | `✅⁺` | `src/ipc.rs:75-102` `prepare_socket_path` — **gerçek `connect` probe'u**; canlıysa `AddrInUse`, `ConnectionRefused/NotFound/TimedOut` ise stale → sil (`:104-109`). Ek: `socket_file_identity` (`:228`), `remove_socket_file_if_owned` (`:246`) | `fresh-editor`'ün PID-dosyası yönteminden **daha güçlü**: PID dosyası yarış koşuluna ve PID yeniden-kullanımına açık; connect probe gerçek durumu ölçer. Ayrıca sahiplik kontrolü ile başkasının soketini silme koruması var |

---

## ⚠️ BULGU — `sun_path` guard'ı API socket yolunu kapsamıyor (doğrulanmamış risk)

**İddia.** `PI15` disiplini `src/remote/unix.rs`'de mükemmel uygulanmış ama
`session.rs::api_socket_path_for` bu guard'dan **geçmiyor**; izin verilen en uzun session adıyla
`sockaddr_un.sun_path` sınırı aşılabilir.

**Hesap** (release, Linux, `XDG_CONFIG_HOME` set değil):

```
/home/<user>/.config/herdr/sessions/<session-name>/herdr.sock
   7 + len(user) + 24                + ≤64          + 11      = 106 + len(user)
```

`src/session.rs:13` `MAX_SESSION_NAME_LEN = 64`, `:429` bunu doğruluyor.
→ **macOS limiti 104**: `len(user)` ne olursa olsun aşılır.
→ **Linux limiti 108**: `len(user) ≥ 2` ise aşılır.
→ Debug derlemede dizin adı `herdr-dev` (+4 byte) ile durum kötüleşir.
→ `XDG_CONFIG_HOME` derin bir yola işaret ediyorsa marj tamamen kaybolur.

**Neden `doğrulanmamış`.** Bu bir *hesap*, çalıştırılmış bir bind denemesi değil. Gerçek
başarısızlık için 64 karakterlik bir session adı gerekir — pratikte nadir, ama session adı
branch/worktree adından türetiliyorsa erişilebilir. `interprocess`'in bu durumda ne hata
döndürdüğü de ölçülmedi.

**Kapanış testi (önerilen, henüz yazılmadı).**
`remote/unix.rs:3047`'deki `local_forward_socket_path_fits_in_sun_path` testinin eşleniği:
`MAX_SESSION_NAME_LEN` uzunluğunda bir adla `api_socket_path_for` çağrılıp sonucun
platform sınırına sığdığını iddia eden bir test. Test kırmızı yanarsa gerçek bir bug'dır;
yeşil yanarsa bu bulgu kapanır.

---

## ⚠️ BULGU 2 — Windows named-pipe'ta hiçbir erişim bariyeri yok (doğrulanmamış risk)

**İddia.** herdr'ın Windows IPC yolunda ne DACL, ne sahip doğrulaması, ne de dosya-izni karşılığı
var; pipe adı tahmin edilebilir. Başka bir yerel hesap adı **kapıp** (squat) herdr istemcisine
sahte veri besleyebilir.

**Kanıt (herdr tarafı):**
- `src/ipc.rs:283-285` — `#[cfg(windows)] restrict_socket_permissions(_path, _mode) -> Ok(())`
  yani **no-op**. Unix'teki `0600` karşılığı Windows'ta yok.
- `src/ipc.rs:60-70` — listener `path.to_string_lossy().to_ns_name::<GenericNamespaced>()` ile
  **varsayılan DACL**'le açılıyor; ad, socket yolundan türediği için tahmin edilebilir.
- `reclaim_name(false)` sunucu tarafını korur (isim doluysa bind **hata verir**, sessizce
  devralmaz) — ama **istemci** tarafında bağlandığı pipe'ın kime ait olduğunu doğrulayan
  hiçbir kontrol yok.

**Kanıt (prior-art tarafı — çözüm mevcut ve yazılı):** `[gwm-cli]` `src/daemon.rs:1420-1460`
tam bu tehdidi adlandırıp çözüyor: *"`\\.\pipe\` names are first-come-first-served and
predictable, so another local account could squat `gwm-<user>.sock` with a permissive DACL and
feed forged worktree data to the statusline and every other consumer"*. Çözümü: `D:P` +
OWNER RIGHTS DACL, ve **istemci tarafında** bağlı kernel nesnesinden sahip SID'i okuyup
karşılaştırma (PID-yeniden-kullanım yarışı yok, `fails closed`).

**Neden `doğrulanmamış`.** Windows'ta squat denemesi **yapılmadı**. Gerçek sömürülebilirlik
yarış penceresine bağlı (saldırganın herdr sunucusundan önce ismi alması gerekir). Ayrıca
`interprocess` 2.x'in varsayılan DACL'ini okumadım — kütüphane zaten daraltıyor olabilir.

**Kapanış adımı.** (a) `interprocess` 2.4.2'nin Windows listener DACL varsayılanını doğrula;
daraltmıyorsa (b) `gwm-cli` modelini uygula: owner-only DACL + istemci tarafı `verify_server_owner`.
herdr'ın `platform/` izolasyon ilkesine uyar; `[gwm-cli]` **MIT** olduğu için kod atıfla alınabilir.

---

## Öncelik değerlendirmesi (öneri, karar değil)

| Sıra | İş | Gerekçe | Risk |
|---|---|---|---|
| 1 | `interprocess` Windows DACL varsayılanını doğrula (BULGU 2 adım a) | Tek okuma/deney; sonuç "risk yok"sa 2. sıra düşer, "var"sa güvenlik işi olur | Yok — araştırma |
| 2 | `PI16` Windows: owner-only DACL + istemci `verify_server_owner` | Tek net güvenlik gap'i; `gwm-cli` (MIT) çalışan modeli veriyor, kod atıfla alınabilir | Orta — `platform/windows.rs` + test |
| 3 | `sun_path` testi (BULGU 1) | Ucuz (tek test), mevcut disiplinin eksik kapsamını kapatır, sonucu net | Yok — test eklemek |
| 4 | `PI16` Unix: `peer_creds` politikası | Dosya izni azaltıyor ama bağlantı kimliği değil; `running-process` modeli hazır | Orta |
| 5 | `PI5` `try_wait` + release ayrımı | Uzun ömürlü session store'da zombie/fd sızıntısı önler | Düşük |
| 6 | `PI18` protokol crate ayrımı | Guardrail'i derleyiciye çivilerdi | Yüksek — workspace'e geçiş, upstream fark yüzeyini büyütür (fork disiplini!) |
| 7 | `PI4` batch window | **Önce ölç.** Pattern zaten tek-kaynak | Düşük ama getirisi kanıtsız |

> **Fork disiplini hatırlatması.** herdr bir fork; `PI18` gibi yapısal değişiklikler upstream
> merge yüzeyini büyütür. `behaviors/` protokolü gereği fork-özel davranış **adıyla bir teste
> sahip olmalı**, yoksa sonraki merge sessizce silebilir.

---

## Sonuç

herdr **havuzun ortalamasının üstünde**. 19 pattern'in 6'sında (`PI1`, `PI3`, `PI12`, `PI13`,
`PI15`, `PI19`) referans projelerin hepsinden daha iyi bir uygulama var — özellikle
*testle çivilenmiş* olmaları (fd sayımı, sun_path fit) havuzda eşi olmayan bir disiplin.

Gerçek eksikler dar: **`PI16` (peer-cred)** tek net güvenlik gap'i, **`PI5`** hijyen,
**`PI18`** mimari-zorlama. `PI4` uygulanmamalı — önce ölçülmeli.

*herdr koduna bu turda hiçbir değişiklik yapılmadı.*
