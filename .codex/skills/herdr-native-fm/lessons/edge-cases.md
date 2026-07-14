# Herdr Native-FM Edge Cases

| Situation | Solution | Date |
|-----------|----------|------|
| A warned MCP has no direct `mcp_servers` entry in `config.toml` | Inspect enabled plugin declarations and the matching plugin MCP manifest; plugins can inject named MCP servers such as GitHub | 2026-07-14 |
| A `Type=simple` MCP proxy initializes every named stdio backend serially before opening its listener | Size readiness from a measured isolated cold start plus bounded headroom; keep exact server-set and critical MCP probes so a larger timeout cannot turn a genuine hang into a false green | 2026-07-14 |
| A throwaway terminal host window exists while a Rust probe is still compiling, so an early screenshot captures startup text rather than graphics evidence | Pre-build the ignored probe or wait for its explicit on-screen marker before capture; inspect the capture and reject compile/startup-only images | 2026-07-14 |
| A Ratatui buffer test derives a cell coordinate with `str::find` from a row containing Unicode separators | Convert the byte offset to a character count before indexing terminal cells; UTF-8 byte positions are not screen-cell positions | 2026-07-14 |
