# FAZ-F (flicker) + FAZ-M (Mac hoparlöründen ses) — PRD & görev/TN planı (S51, 16:3x)

## Bağımlılık zinciri (ölçülmüş)
```
FAZ-F: fix f3c6643b (in-place retransmit) ──► iniş (16:35 tur-1) ──► auto-deliver ──► handoff (client'lar kopar) ──► TN-F*
FAZ-M: motor ✓ (CoreAudioSink platform/macos.rs:1322; köprü media/sink.rs:288/302)
       capability ✓ (client_capabilities(sink_available) → AUDIO_SINK=OPUS; fan-out TP-MEDIA-CAP-07: is_full_app_client && OPUS)
       Mac binary ✓ (~/herdr, Mach-O x86_64, smoke: client connected)
       ──► M1 kullanıcı Mac TUI (SemanticFrame=App mode) ──► M2 müzakere kanıtı ──► M3 P3 besleyici ──► M4 KULAK
```

## FAZ-F görev/TN
| TN | senaryo | beklenen | NEDEN |
|---|---|---|---|
| TN-F1 | teslim bekleyicisi | deliver.log yeni `teslim tamam: <merge-sha>` | fix ancak canlı server'da etkili (iniş ≠ teslim, TZK-9) |
| TN-F2 | video play (ben başlatırım) + kullanıcı gözü | yanıp sönme YOK; akıcı video | şikâyet katmanı = ekran; kanıt o katmandan (observed-defect §5) |
| TN-F3 | agent-geçişi ×5 (browser+2 client repro) | kalıntı frame 0 | STABLE-01'in asıl amacı; regresyonsuz kapanış (V4.TN-5) |

## FAZ-M görev/TN
| görev | ne | TN | beklenen + NEDEN |
|---|---|---|---|
| M1 | Kullanıcı Mac'te: `~/herdr --remote user@100.64.0.1` | TN-M1 | server log `client connected ... SemanticFrame` (App-mode şart: TerminalAnsi is_full_app_client=false → sese giremez) |
| M2 | Müzakere kanıtı | TN-M2 | Mac `~/.config/herdr/herdr-client.log`: media satırı/`negotiated`; declined ise kök CoreAudio probe (macos.rs:1313) |
| M3 | P3: `pane-audio-source.py --pane w1:p10` + tb sink-input MUTE (pactl) | TN-M3 | P3 log: sink-input eşleşti + kare sayacı akıyor; mute=çift-ses önlemi (tb lokal pulse'a da çalıyor) |
| M4 | KULAK — Mac hoparlörü | TN-M4 | tek geçerli son kanıt (kullanıcı) |
| M5 | kapanış stats | TN-M5 | Mac client log `media stream closed ... played>0 underruns=~0` (PLAYBACK-08) |

## Riskler / notlar
- Handoff her teslimde TÜM client'ları düşürür → kullanıcıya her teslim sonrası "yeniden bağlan" adımı (S50-3 dersi).
- Çift-ses: local TUI de OPUS-capable (laptop ffplay→Dell dock) → Mac testi sırasında laptop çıkışından da çalabilir; tb mute + gerekirse dock volume notu.
- pane doğum-anı pixel 0×0 (backend/unix.rs:19) ayrı görev **PANE-PX** (bugün ölçüldü; resize workaround canlı).

## STRATEJİ EKİ (17:5x — dış referans tablosu diff'i sonrası)
Sıra: (1) PAKET feat/graphics-stream-package (kimlik çifti+retained, 14/14) — teslim ÖNCESİ izole prof ölçümü ZORUNLU (full vs retained sayaçları + encode süresi); teslim kararı KULLANICIDA. (2) M-SES canlı (P3 --sink-input→CoreAudio kulak) + M-BELL (Mac bildirim sesleri) + pixel-mouse uzak-guard (KITTY_WINDOW_ID→pixel_mouse=true; sgr_pixels motion seli Mac kasması adayı). (3) F1.5: server-içi pw_stream yakalama (P3 python emekli; eşleşme acıları biter) + Opus inband_fec. (4) V-LANE/F2: video→media şeridi + dirty-rect (terminal-içi önizleme sınıfı). (5) F3 QUIC datagram + sidecar player (tam video; ses HOL'u için tek başına gereksiz — kredi yeterli, 0,46 MB/dk).
Reddedilenler (dış tabloyla mutabık): PA/PW ağ tüneli · HTTP-akış (1-2s tampon) · snapcast sabit-gecikme · ffplay-birincil-sink.
