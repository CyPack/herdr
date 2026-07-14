# Herdr Native-FM Edge Cases

| Situation | Solution | Date |
|-----------|----------|------|
| A warned MCP has no direct `mcp_servers` entry in `config.toml` | Inspect enabled plugin declarations and the matching plugin MCP manifest; plugins can inject named MCP servers such as GitHub | 2026-07-14 |
| A `Type=simple` MCP proxy initializes every named stdio backend serially before opening its listener | Size readiness from a measured isolated cold start plus bounded headroom; keep exact server-set and critical MCP probes so a larger timeout cannot turn a genuine hang into a false green | 2026-07-14 |
