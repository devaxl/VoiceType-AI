# Contributing to VoiceType AI

Thanks for your interest in VoiceType AI! It's an open-source, Windows-first
desktop voice-dictation app: a global hotkey captures your microphone, sends the
audio to OpenAI for speech-to-text, refines the transcript with an LLM, and
injects the cleaned-up text into whatever field is focused. It's built with
**Tauri v2** (a Rust core) and a small **React + Vite** settings UI, and it's
maintained by [Devaxl](https://devaxl.com).

This guide covers setting up a dev environment, the project layout, how to run
the app, our coding conventions, and how to file issues and open pull requests.
We aim to keep contributing low-friction — if anything here is unclear or out of
date, please open an issue.

---

## Code of conduct

Be kind, be constructive, assume good faith. We want VoiceType AI to be a
welcoming project. Harassment or disrespectful behavior isn't tolerated in
issues, PRs, or any other project space.

---

## Before you start

A few things worth knowing up front:

- **You bring your own OpenAI API key.** The app has no central backend — each
  user supplies their own key, which is stored in the OS keychain (never in
  plaintext). You'll need a key to actually exercise the end-to-end pipeline.
- **Status is "working v0."** The core loop, background/tray mode, safe-inject
  guards, the rebindable hotkey, named refinement profiles, custom vocabulary,
  and persisted settings all work and build on a dev machine. It is **not yet
  packaged or code-signed**, and macOS is built in CI but not yet validated on a
  notarized build. See the [README](README.md) and [PRD.md](PRD.md) for the full
  picture and the known v0 gaps.
- **Anthropic support is on the roadmap, not implemented.** Today the only
  provider is OpenAI (`gpt-4o-mini-transcribe` for STT, `gpt-4.1-nano` for
  refinement). Please don't describe Anthropic as a current feature in docs or
  UI copy.

---

## Setting up a dev environment

### Prerequisites (Windows — the primary target)

- **[Rust](https://rustup.rs)** (stable toolchain). The **MSVC** toolchain
  (Visual Studio C++ Build Tools) is recommended. The **GNU** toolchain also
  works, but it needs a clone path **with no spaces**; if yours has one, add a
  local (gitignored) `src-tauri/.cargo/config.toml` that sets `build.target-dir`
  to a space-free path, e.g.:
  ```toml
  [build]
  target-dir = "C:/voicetype-target"
  ```
- **[Node.js](https://nodejs.org) LTS** (CI uses Node 20).
- **WebView2** — already preinstalled on Windows 11.

### macOS

macOS is built in CI (`macos-latest`) but is **not yet validated end-to-end** —
the audio-input entitlement, `getUserMedia` capture, and the live
Accessibility/Input-Monitoring permission check are still to come (see PRD
§9.1–9.3). Contributions to stand this up are very welcome; just be aware the
macOS path isn't a finished, runnable target yet.

### Clone, install, run

```bash
git clone https://github.com/devaxl/VoiceType-AI.git
cd VoiceType-AI
npm install
npm run tauri dev
```

Then, in the settings window: paste your **OpenAI API key** and save. Put your
cursor in any text field (Slack, Notepad, VS Code…), press **Alt+Shift+D**,
speak, press it again, and the refined text should appear.

### Building installers

```bash
npm run tauri build
```

Builds are **not code-signed** yet, so Windows SmartScreen may warn on first run
and macOS builds are unsigned (right-click → Open to bypass Gatekeeper). Signing
comes with the distribution milestone; prefer the MSVC toolchain for shipping
builds. Cross-platform installers are also produced automatically by CI when you
push a version tag — see [`.github/workflows/release.yml`](.github/workflows/release.yml).

---

## Project layout

The Rust core lives in a single crate under `src-tauri/src/`, organized by
module. (The PRD describes a future multi-crate workspace; we'll split once the
trait boundaries stabilize — please keep new code within the existing module
structure for now.)

```
.
├─ PRD.md                  # product spec + locked decisions
├─ README.md
├─ index.html, src/        # React + Vite settings UI
├─ docs/
│  └─ data-handling.md     # privacy disclosure surfaced in Settings
├─ src-tauri/
│  ├─ tauri.conf.json      # Tauri v2 config
│  ├─ capabilities/        # window permission capabilities
│  └─ src/
│     ├─ lib.rs            # app builder, plugin + hotkey + tray + HUD wiring
│     ├─ state.rs          # Idle → Recording → Processing state machine
│     ├─ hotkey.rs         # global shortcut + toggle logic
│     ├─ audio.rs          # cpal capture (dedicated thread) + WAV encode
│     ├─ http.rs           # shared HTTP client + retry/backoff
│     ├─ stt.rs            # OpenAI transcription client
│     ├─ refine.rs         # OpenAI chat-completions refinement client
│     ├─ inject.rs         # clipboard-paste + direct-typing injection (enigo + arboard)
│     ├─ winfocus.rs       # Win32 focus-verify + secure-field detection
│     ├─ pipeline.rs       # orchestrates STT → refine → guards → inject
│     ├─ secrets.rs        # API key in OS keychain (keyring)
│     ├─ persist.rs        # config load/save (JSON in the app-config dir)
│     ├─ commands.rs       # Tauri commands exposed to the UI
│     └─ tray.rs           # system-tray menu
└─ .github/workflows/      # ci.yml (build check) + release.yml (installers)
```

A good way to find your bearings: trace one full activation through
`hotkey.rs → state.rs → audio.rs → pipeline.rs → stt.rs → refine.rs → inject.rs`.

---

## Coding conventions

CI compiles the app on a **Windows + macOS** matrix on every push to `main` and
every pull request, so your change must build on both. We also ask contributors
to run **rustfmt** and **clippy** locally before pushing — clean formatting and
no new warnings keep reviews fast.

**Rust** (from `src-tauri/`):

```bash
cargo fmt --all                              # format
cargo clippy --all-targets -- -D warnings    # treat warnings as errors
cargo build
```

- Format with `rustfmt` before committing.
- Keep clippy clean; fix lints rather than `#[allow(...)]`-ing them unless
  there's a clear, commented reason.
- Keep the latency-sensitive path (audio, hotkey, injection, state machine) in
  Rust. The webview is for settings UI only.
- Respect the safety guards — focus-verify before inject, full clipboard
  save/restore, the secure/password-field typing bypass, the
  hallucination/empty-audio filter, and the cancel-prior concurrency policy.
  These exist for good reasons (see PRD §8–§9); if you touch them, explain why in
  the PR.
- **Never make the app synthesize Enter / auto-send.** This is a deliberate
  safety guarantee (PRD §6.5).
- Keep secrets out of logs. Never log API keys, audio, transcripts, or refined
  text.

**Frontend** (settings UI):

```bash
npm install
npm run build     # CI builds the frontend; make sure it compiles
```

- Match the existing React + Vite style in `src/`.
- Don't introduce heavy dependencies for the settings UI without discussion — a
  small footprint is a project goal.

---

## Filing issues

- **Search first** to avoid duplicates.
- For **bugs**, include: OS + version, app version / commit, the Rust toolchain
  (MSVC or GNU), steps to reproduce, what you expected vs. what happened, and any
  relevant (secret-scrubbed) logs. Injection bugs are especially helpful when you
  name the **exact target app** (e.g. "Slack desktop," "VS Code integrated
  terminal," "Outlook web") since reliability there is app-specific.
- For **feature requests**, describe the problem you're trying to solve, not just
  the solution. Check the PRD roadmap (§13) first — some things are already
  planned.
- **Never paste secrets** — API keys, audio, or real transcript content — into an
  issue.

---

## Opening a pull request

1. **Fork** the repo and create a topic branch off `main`
   (e.g. `fix/clipboard-restore-on-panic`).
2. For anything non-trivial, **open an issue first** to discuss the approach — it
   saves everyone time.
3. Keep PRs **focused**: one logical change per PR.
4. Make sure it builds (`cargo build`, `npm run build`) and is formatted
   (`cargo fmt --all`) with no new clippy warnings.
5. Write a clear PR description: what changed, why, and how you tested it. If it
   touches injection or a safety guard, say which apps/fields you tested against.
6. Update docs (README / PRD / `docs/`) when behavior or setup changes.
7. By contributing, you agree your contributions are licensed under the project's
   [MIT License](LICENSE).

A maintainer will review as soon as we can. Friendly, iterative feedback is the
norm — don't be discouraged by review comments.

---

## Security

Please **do not** open public issues for security vulnerabilities. VoiceType AI
synthesizes keystrokes and holds an API key in the OS keychain, so we take
security seriously — see [SECURITY.md](SECURITY.md) for how to report privately.

---

VoiceType AI is built by [Devaxl](https://devaxl.com). If you like our work, also
check out [nrtur](https://nrtur.io). Thanks for contributing!
