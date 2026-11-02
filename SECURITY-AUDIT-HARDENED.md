# Handy — security audit and hardened fork

Audit date: 2 November 2026 · Upstream: github.com/cjpais/Handy @ `82dfab9` (v0.10.0) · Branch: `hardened`

Scope: Tauri configuration and capabilities, all 134 IPC commands' trust boundary, every process spawn,
network client, file write and archive path in the Rust backend (≈36,000 lines), the React frontend's HTML
sinks, and both dependency trees (852 Rust crates via RustSec, 353 JS packages via the GitHub advisory DB).

## Findings and fixes

| # | Severity | Finding | Fix in this branch |
|---|---|---|---|
| H1 | High | No Content Security Policy (`csp: null`): any script that reached the window had all 134 backend commands. | Strict CSP: scripts and connections only to the app itself (`connect-src ipc:`), no frames/objects/forms; `freezePrototype` on. Dev CSP unchanged. |
| H2 | High | Asset protocol scope `**`: the window could read **any file on disk** (SSH keys, browser data) through `asset://`; fs scope covered all app data incl. the settings file. | Asset and fs scopes limited to the recordings folder (static `$APPDATA/recordings`, plus the real folder at runtime so portable mode works). |
| H3 | High (chain) | A script in the window could set paste method "external script" to any program (e.g. `powershell.exe`) and aim AI post-processing at its own server, so every dictation would run attacker commands. The option is hidden on Windows/macOS in the UI but the backend accepted it there. | External-script paste is Linux-only in the backend. On Linux the script must be an absolute path to an executable the user owns, not group/world-writable, outside system directories — checked when it runs. |
| M1 | Medium | `store:default` let the window rewrite `settings_store.json` directly, bypassing every backend check. The UI never uses it. | Capability removed. |
| M2 | Medium | Custom AI endpoints accepted plain `http://` to any host, sending the API key and transcript unencrypted; redirects could downgrade to http. | https required except localhost / private-network hosts; redirects that downgrade to public http are refused; max 5 redirects. |
| M3 | Medium | `get_audio_file_path` joined any string (`..\..\x`) onto the recordings folder. | Only bare file names accepted; missing audio returns a clear error. |
| M4 | Medium | Vulnerable TLS stack: rustls 0.23.36 / rustls-webpki 0.103.9 (distrusted intermediates accepted, name-constraint bypasses, key-exchange downgrade), webpki roots with distrusted emSign CAs, h2 DoS, tar symlink chmod. | Updated in-semver: rustls 0.23.46, webpki 0.103.16, roots 1.0.11, h2 0.4.20, tar 0.4.46. RustSec: 21 → 7, none reachable on Windows at runtime (see residual). |
| M5 | Medium (fork) | The built-in updater installs the official release, which would silently replace this build and undo every fix. | Updater forced off (UI shows "Update Checking Disabled"), its capability removed, no updater artifacts. |
| L1 | Low | Post-processing API keys stored in plain text in `settings_store.json`. | Upgrade: keys live in Windows Credential Manager / macOS Keychain; existing plain-text keys migrate on first run; Linux keeps the old behaviour. |
| L2 | Low | 55 JS advisories (Vite, Rollup, PostCSS, etc.) — build tooling, not shipped. | Updated within semver; Tauri JS packages pinned to the Rust crate versions. 55 → 2. |

Good practices already present upstream (kept): SHA-256 verification of catalog model downloads, bundled
model catalog, transcripts excluded from Windows Clipboard History and cloud clipboard, no transcript text in
logs, API keys redacted from debug output, signed updates (upstream), no listening network services, React
escaping everywhere (the only Markdown renderer has raw HTML off).

## Upgrades (requested: "make it better")

1. **Offline lock** (Settings → Advanced → Privacy & security): blocks model downloads and any AI
   post-processing except a server on this computer. Verified: a download attempt is refused with
   "Offline lock is on".
2. **Auto-wipe recordings** (on by default): keeps audio only for the newest dictation and saved entries;
   older audio is deleted when the next dictation is stored; all text is kept.
3. **API keys in the OS vault** (L1 above).

## Verification

- `cargo test --lib`: 346 tests (338 upstream + 8 new) all pass as a normal user. One upstream test,
  `test_delete_reports_a_file_it_cant_remove`, fails only when run as root (root can delete read-only files).
- New tests: path validation, endpoint policy, external-script guard, auto-wipe, vault fallback.
- Frontend: `tsc` + Vite build, ESLint, Prettier, translation completeness (26 locales) all pass.
- Built with `custom-protocol` (production asset loading, CSP enforced) and run on Linux/WebKitGTK:
  onboarding, settings, models and the new Privacy & security group render; offline lock blocks downloads.

## Residual risk (not fixed here)

- Windows installer is **unsigned**: SmartScreen warns; verify the artifact came from your fork's Actions run.
- **No automatic updates**: pull upstream into the fork and rebuild to get future fixes.
- Credential-vault code is compiled for Windows by CI but was runtime-tested here only via the Linux fallback.
- `zip` 4.6.1 (RustSec 2026-0367/0368/0369/0429) remains inside the now-disabled updater; `quick-xml` and
  `rkyv` advisories are build-time only. JS: `minimatch` (linter only), `zod` 3.x (no fix in v3; validates
  only the app's own data).
- History database and kept recordings are unencrypted in your user profile; API keys are still held in
  the window's memory while settings are open.
- New setting labels are English in all 26 languages.
