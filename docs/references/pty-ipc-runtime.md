---
doc: herdr-references-pty-ipc-runtime
domain: pty-ipc-runtime
created: 2026-07-25
status: canonical — çıplak iddia yok; her giriş tier + confidence taşır (evidence-propagation uyumlu)
pattern_id_space: PI1–PI19 (pattern) + PA1–PA11 (anti-pattern)
git_note: >
  /docs/* herdr .gitignore'da IGNORED → bu dosya LOKAL yaşar, upstream'e/PR'a SIZMAZ.
  Makine kopyası: .cartography/pty-ipc-runtime-SYSTEM-MAP.json + ~/.cartography/ (aynası).
  Kaynak kod havuzu ~/.cartography/refpool/ altında KALICI klonlar hâlinde durur ve
  codebase-memory-mcp'ye indekslidir — bu registry'nin her satırı canlı koda çözülebilir.
agentic_triggers:
  - "pty · pseudo-terminal · portable-pty · openpty · conpty · pty spawn · pty resize · pty eof"
  - "ipc · unix socket · named pipe · interprocess · socket path · framing · length-prefix"
  - "daemon · server/client ayrımı · detach · attach · reattach · session persist"
  - "protokol versiyonu · handshake · capability · wire format · bincode · protobuf"
  - "prior art · bu zaten var mı · kim nasıl yapmış · referans proje · örnek proje"
related:
  - docs/patterns/pty-ipc-runtime.md              # pattern kataloğu (bu registry'nin damıtılmış hâli)
  - docs/references/README.md                     # domain index
  - .cartography/pty-ipc-runtime-SYSTEM-MAP.json  # evidence graph
  - ~/.cartography/refpool/                       # klonlanmış kaynak havuzu (17 repo)
  - ~/.claude/skills/rust-dev/                    # GLOBAL Rust katmanı
---

# DOMAIN: pty-ipc-runtime — Referans Registry

> **Kapsam.** herdr'ın üç çekirdek bağımlılığının (`portable-pty` · `interprocess` · `tokio`)
> production'da nasıl birlikte kullanıldığına dair prior-art havuzu. Amaç: PTY yaşam döngüsü
> ve IPC protokol kararlarını sıfırdan icat etmeden, kanıtlı kaynağa dayandırmak.
>
> **Tier sözlüğü:** `source_code` (klonlanmış canlı kod — havuzdaki en güçlü kanıt) ·
> `official` (crate'in kendi docs/örnekleri) · `design_doc` (projenin kendi tasarım
> dokümanı — yazarın niyet beyanı) · `executable` (çalıştırılmış komut çıktısı) ·
> `derivative` (herdr'dan türemiş — BAĞIMSIZ KANIT DEĞİL, aşağıdaki uyarıya bak).

---

## ⚠️ Bağımsızlık uyarısı — sahte konsensüs riski

`zynk` **herdr'ın yeniden markalanmış fork'udur** (kendi README'sinde beyan ediyor;
`src/` altındaki 214 `.rs` dosyasının **187'si herdr ile ortak**). Bir pattern'i "iki
bağımsız proje de böyle yapmış" diye doğrularken **zynk sayılmaz** — o bizim kendi
kodumuzun aynası. `orbt` ise kod-fork değil ama CLAUDE.md'sinde "herdr heritage" diyor
(tasarım etkisi var, uygulaması bağımsız: kendi `orbt-protocol` crate'i).

Bu yüzden aşağıdaki tablo `bağımsızlık` sütunu taşır. θ-kuralı (2 BAĞIMSIZ kaynak ≥0.7)
uygulanırken **yalnız `bağımsız` satırlar** sayılır.

---

## Havuz envanteri (17 repo — 2026-07-25 klonlandı, hepsi codebase-memory-mcp'de indeksli)

Seçim yöntemi **crates.io reverse-dependency kesişimi**: PTY ailesi (`portable-pty`,
`pty-process`, `expectrl`, `rexpect`, `ptyprocess`, `termwiz` → 609 ters-bağımlı) ∩
IPC ailesi (`interprocess`, `parity-tokio-ipc`, `ipc-channel` → 228 ters-bağımlı) =
**15 crate**; buna kanonik büyükler (`zellij`, `sshx`) eklendi.

| Etiket | Repo | SHA | Tier | Conf | Bağımsız? | tokio | Neden havuzda |
|---|---|---|---|---|---|:-:|---|
| `[oly]` | `slaveOftime/open-relay` | `741ba42` | source_code | 0.9 | ✅ | ✅ | **En yakın mimari eşleşme**: kalıcı PTY session'lı daemon, detach/attach, client-yokken capability responder |
| `[orbt]` | `linuszz/orbt` | `186c11c` | source_code + design_doc | 0.85 | ⚠️ etkilenmiş | ✅ | Ayrı `orbt-protocol` crate'i (tokio-free), length-prefixed bincode, socket-path önceliği. ⚠️ `CLAUDE.md`'si **aspirational** — SO_PEERCRED iddiası kodda YOK (bkz. `PI16` dersi); `design_doc` satırlarını koda çözmeden kullanma |
| `[running-process]` | `zackees/running-process` | `da50923` | source_code | 0.9 | ✅ | ✅ | PTY backend trait soyutlaması (posix / ConPTY / portable-pty passthrough) + **`PI16`'nın tek gerçek implementasyonu** (`PeerCredentialPolicy`, testli, platform-farkı belgeli) |
| `[zellij]` | `zellij-org/zellij` | `812ad86` | source_code | 0.95 | ✅ | ✅ | Kanonik client/server ayrımı; kendi PTY'si + `interprocess` IPC; `sun_path` limit guard'ı |
| `[sshx]` | `ekzhang/sshx` | `dd42496` | source_code | 0.9 | ✅ | ✅ | `portable-pty` KULLANMAZ — ham `nix::pty::openpty` + `login_tty` + `TIOCSWINSZ`; alternatif taban |
| `[tenex]` | `Mockapapella/tenex` | `fda7d37` | source_code | 0.8 | ✅ | ❌ | AI-agent multiplexer; `mux/server/session.rs` reap semantiği |
| `[fresh-editor]` | `sinelaw/fresh` | `4610966` | source_code | 0.85 | ✅ | ✅ | **Stale-socket canlılık testi** (`cleanup_if_stale`) + ayrı data/control socket + PID dosyası |
| `[kode-bridge]` | `KodeBarinn/kode-bridge` | `de31d0f` | source_code | 0.85 | ✅ | ✅ | HTTP-over-IPC: connection pool, retry, metrics — protokol katmanı olgunluğu |
| `[mprocs]` | `pvolok/mprocs` | `d1966e7` | source_code | 0.8 | ✅ | ✅ | Çoklu-process TUI; `rustix` ile ham PTY |
| `[tui-term]` | `a-kenji/tui-term` | `e1ebdd0` | source_code + official | 0.85 | ✅ | ✅ | ratatui PTY widget'ı — pseudo-terminal → widget sınırı |
| `[skim]` | `skim-rs/skim` | `cbfb7fa` | source_code | 0.8 | ✅ | ✅ | 1.87M indirme; olgun hata yönetimi, `interprocess` idiomları |
| `[ptywright]` | `utensils/ptywright` | `0586657` | source_code | 0.8 | ✅ | ❌ | PTY üzerinden interaktif TUI **sürme/test etme** — test altyapısı için |
| `[shell-compose]` | `pka/shell-compose` | `914d874` | source_code | 0.75 | ✅ | ❌ | Minimal (1.7k LOC) background job runner + IPC — sade referans |
| `[zwire-host]` | `MenkeTechnologies/zwire-host` | `af77c51` | source_code | 0.7 | ✅ | ❌ | Chrome native-messaging köprüsü + PTY — farklı transport açısı |
| `[gwm-cli]` | `kbrdn1/gwm-cli` | `7212406` | source_code | 0.7 | ✅ | ❌ | git worktree manager (herdr worktree-lens ile komşu alan) |
| `[bohay]` | `RizRiyz/bohay` | `9d16cac` | source_code | 0.7 | ✅ | ❌ | Newline-delimited JSON IPC (bincode'a alternatif framing) |
| `[zynk]` | `dzevs/zynk` | `3d10a90` | **derivative** | 0.5 | ❌ **FORK** | ✅ | herdr fork'u — bağımsız kanıt DEĞİL; değeri **fork-diff**: aynı tabandan farklı evrim |

## Crate seviyesi kaynaklar (canlı API doğrulaması)

| Etiket | Kaynak | Tier | Conf | Desteklediği pattern | Konu |
|---|---|---|---|---|---|
| `[portable-pty-src]` | `~/projects/herdr/vendor/portable-pty/` (0.9.0, herdr'da vendored) + `refpool/zynk/vendor/portable-pty/examples/` | source_code + official | 0.95 | PI1, PI2, PI3 | `MasterPty` trait sözleşmesi; `bash.rs`/`whoami_async.rs` örnekleri `drop(pair.slave)`'i açıkça gösterir |
| `[crates-revdep]` | `crates.io/api/v1/crates/{crate}/reverse_dependencies` | executable | 1.0 | — | Havuz seçim yöntemi; 2026-07-25 çekildi (portable-pty 423 · interprocess 168 ters-bağımlı) |
| `[interprocess-2x]` | `interprocess` 2.4.2 `Name` API (`to_fs_name::<GenericFilePath>`) | official | 0.9 | PI11, PA-API | 2.x'te `connect`/`bind` artık `&str` almaz — 1.x örnekleri derlenmez |
| `[herdr-live]` | `~/projects/herdr/src/{pty,ipc.rs,protocol,server,client}` | source_code | 0.95 | tümü | Karşılaştırma tabanı — pattern'in herdr'da zaten var olup olmadığı buradan doğrulanır |

## Araç katmanı

| Etiket | Kaynak | Tier | Conf | Konu |
|---|---|---|---|---|
| `[refpool-index]` | `~/.cartography/refpool/` — 17 repo, hepsi `codebase-memory-mcp` `moderate` modda indeksli | executable | 0.95 | Sembol/pattern çözümü: `search_graph(project="home-user-.cartography-refpool-<ad>")`. 2026-07-25 indeksleme sayıları SYSTEM-MAP'te |
| `[pty-ipc-pool-classification]` | `~/.cartography/pools/pty-ipc/` — `taxonomy.json` (9 kategori ailesi) · `sources.json` (17 kayıt) · `classifications/P01–P17.json` (40 claim) · `pattern-index.json` (PI→repo ters indeks) | executable | 0.9 | 2026-07-26. `ratatui` pool'unun şema kardeşi. Şema doğrulandı: taxonomy dışı kategori YOK; 19/19 pattern örneklendi |

## ⚖️ Yeniden-kullanım politikası (lisans kapısı — kod almadan ÖNCE oku)

herdr **AGPL-3.0-or-later + ticari** ikili lisanslıdır (`LICENSE:1-7`). Ticari kolu satabilmek
telif hakkının herdr'da veya izin-verici lisansta olmasını gerektirir → **copyleft kaynaklardan
kod kopyalanamaz**, yalnız tasarım okunur.

| Politika | Repo | Ne yapılabilir |
|---|---|---|
| `code-reusable-with-attribution` (13) | `oly` `running-process` `zellij` `sshx` `kode-bridge` `tenex` `mprocs` `tui-term` `skim` `ptywright` `shell-compose` `zwire-host` `gwm-cli` | MIT/Apache/BSD — atıf + lisans dosyası ile kod alınabilir |
| `design-reference-only` (3) | `orbt` (AGPL) · `bohay` (AGPL) · `fresh-editor` (GPL-2.0) | **Kod ALINMAZ.** Yalnız fikir/yaklaşım okunur, sıfırdan yazılır |
| `derivative-of-herdr` (1) | `zynk` (AGPL fork) | Bağımsız kanıt değil; kod alışverişi upstream/fork ilişkisi olarak ayrıca değerlendirilmeli |

> `oly` — en yakın mimari eşleşme — **MIT**. `PI5`/`PI6`/`PI7` gibi en değerli pattern'lerin
> kaynağı yeniden-kullanılabilir tarafta.

---

## Kayıt kuralı (yeni kaynak eklerken)

1. Etiket ver (`[kebab-case]`), tabloya satır ekle — tier + confidence + **bağımsızlık** ZORUNLU.
2. Repo ise: `~/.cartography/refpool/` altına shallow klonla, SHA'yı yaz, `index_repository`
   ile indeksle. Klonlanmamış "duydum ki" kaynağı bu tabloya GİRMEZ.
3. Kaynak bir pattern'i destekliyorsa `docs/patterns/pty-ipc-runtime.md`'deki `PI*` ID'sini yaz.
4. `.cartography/pty-ipc-runtime-SYSTEM-MAP.json`'a claim/evidence olarak işle.
5. herdr'dan türemiş bir projeyse **`derivative`** işaretle — θ-kuralında saydırma.

---
*v1.0.0 — 2026-07-25 · reference-registry 5-adım pipeline'ın Adım-1 artefaktı.*
