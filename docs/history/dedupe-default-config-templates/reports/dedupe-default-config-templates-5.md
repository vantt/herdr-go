# dedupe-default-config-templates-5

[DONE]

install.ps1's config-write block now captures `herdr-go --internal-print-default-config` stdout and writes it via `WriteAllText` + `UTF8Encoding($false)`, replacing the hand-written `ConvertTo-Json` hashtable literal that was missing `agent_presets`. No `Out-File`/BOM-producing write introduced; idempotent only-if-missing guard and `$ConfigFile` path unchanged.

Files touched: `install.ps1`

Full trace/evidence: `.bee/cells/dedupe-default-config-templates-5.json`
