# Privacy (Fase 5)

> Local-first defense-in-depth, not a leak-proof guarantee.

## What is processed locally

- Discovery, scanning, classification and (future) indexing run on your
  machine. No telemetry, no mandatory network, no code execution during
  analysis. An external LLM is only ever an explicit, optional consumer of
  exported context.

## Automatic exclusion

- Path-based secrets (`.env`, `*.pem`, `*.key`, …) are excluded *before* any
  content is read whenever `security.exclude_secrets = true` (default).
- Remaining candidates get a single bounded read (≤ `max_file_bytes`,
  default 512 KiB). The same buffer backs the binary (NUL) check and the
  high-confidence secret detector:
  - PEM `-----BEGIN … PRIVATE KEY-----` markers (generic + RSA/EC/DSA/OPENSSH
    variants);
  - selected provider token shapes (`ghp_|gho_|ghu_|ghs_|ghr_`, `xoxb-|xoxp-|…`,
    `sk_live_|sk_test_|rk_live_|rk_test_`);
  - database URLs with non-empty `user:password@` userinfo for known schemes;
  - explicit `name = "value"` credential assignments with length/composition
    criteria and placeholder filtering.
- Hits are excluded (`Sensitive` for paths, `SecretContent` for content) with
  static reasons only. Values, spans and snippets never enter `ScanReport`,
  warnings, logs, serialized JSON, CLI output or classification reports.
- Oversized files stay `TooLarge` without being read fully; growth between
  the metadata check and the read is treated conservatively as `TooLarge`.
- Symlinks are never followed or read.

## User-configured exclusions

- `security.extra_excludes`: hard gitignore-style path policy (e.g.
  `["**/private/**", "**/*.local"]`). Evaluated before `.gitignore`
  reinclusions and without reading content; `!` negations are rejected.
  Reported as `SecurityExclude`, distinct from the soft `index.extra_ignores`
  (`ExtraIgnore`, reopenable by a valid `.gitignore` whitelist).

## Opt-out

- `security.exclude_secrets = false` explicitly disables automatic secret
  exclusion (path + content). Credential-bearing content may then be
  admitted. This significantly reduces protection. Binary, generated,
  oversized and `extra_excludes` policy still apply.

## Limitations

- Initial pattern selection, not a catalog of all credential types. A lone
  public identifier (e.g. `AKIA…`) is not flagged; AWS-style secrets must
  arrive as assignments. Long random strings without a known shape are not
  flagged (no entropy-only rule).
- Placeholders (`changeme`, `replace_me`, `your_api_key`, `example`,
  `placeholder`, equivalents) suppress assignment/DB findings; adversarial
  placeholders outside the list may still flag or miss.
- Absence of a finding does NOT prove a file is secret-free. Review exports
  before sharing, keep real credentials out of the repo, and never paste
  real tokens into issues or fixtures (use `FAKE…TESTONLY` synthetics).
- The index lives outside the repo (user cache dir) so derived data is not
  committed accidentally; future MCP remains read-only (no shell, no writes).
