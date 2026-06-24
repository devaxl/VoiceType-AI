# Product Requirements Document — VoiceType AI (v1.0, Finalized)

> **Status:** Finalized for build. **Owner:** (PM)  **Last updated:** 2026-06-23
> **Models/providers verified as of:** 2026-06-23 — *model and STT lifecycles here are measured in months; re-validate every release (see §15).*

This document supersedes the original draft PRD. It incorporates a multi-dimensional expert review (architecture, AI pipeline, security/privacy, UX, completeness, delivery) and four locked stakeholder decisions (§0). Three factual corrections from the original draft are folded in: (1) the named STT models were **retired 2026-06-01**; (2) `claude-3-5-sonnet` is retired; (3) the macOS permission design used the wrong primitive and an incomplete persistence strategy. All are corrected below.

---

## 0. Locked Decisions (this build)

| # | Decision | Choice | Consequence |
|---|----------|--------|-------------|
| D1 | API keys / backend | **Per-user client-side keys** (no central proxy) | Keys stored in OS keychain; ZDR enforced at provider-account level; no central rotation/cost-cap (see §9.6, §15). |
| D2 | Platform sequencing | **Windows-first, macOS fast-follow** | Windows internal v1 ships first; macOS gated on Apple enrollment + CI. |
| D3 | Data handling | **Cloud STT + LLM under signed ZDR + no-training** | Audio in memory only; ZDR/no-training + DPA required before launch. |
| D4 | Injection UX | **Blind auto-inject (default)** | Preview is an optional per-profile setting; focus-verify guard, sensitive-field typing fallback, and never-auto-Enter still apply. |

---

## 1. Overview

**VoiceType AI** is a cross-platform (Windows first, macOS fast-follow) desktop utility that runs in the system tray. The user activates a global hotkey from any application, dictates, and the app transcribes the speech, refines it through an LLM using a user-defined instruction profile, and inserts the final text into the active text field.

**Target users:** internal team members (~10–30 people).
**Primary jobs:** draft Slack messages, emails, and documentation quickly without switching context or typing.

**The core loop:** `hotkey → capture audio → speech-to-text → LLM refine → insert into active field`. The latency-sensitive path (audio, hotkey, injection, state) runs in Rust; a small webview hosts only the settings UI.

---

## 2. Goals & Non-Goals

### 2.1 Goals
- G1. Insert refined, ready-to-use text into any focused field from voice, with no app switching.
- G2. Be reliable at the **injection** step — the product's payoff moment — across real apps (Slack, Outlook/Gmail, VS Code, browsers, Office).
- G3. Be unobtrusive: small resident footprint, instant activation, clear recording state.
- G4. Let users shape output via named instruction profiles (casual / formal / bulleted).
- G5. Keep sensitive content safe: cloud calls under ZDR/no-training; audio never persisted; secrets handled deliberately.

### 2.2 Non-Goals (v1)
- N1. **No streaming STT** — batch per-utterance only.
- N2. **No on-device/offline transcription** in v1 (roadmap; see §13).
- N3. **No voice commands / no auto-send** — the app never synthesizes Enter (§6.5).
- N4. **No mobile or web client.** Desktop only.
- N5. **Single user per device.** No multi-tenant profiles on one machine.
- N6. **English-first.** Multilingual is an open item (§15), not a v1 commitment.
- N7. **No central management backend** (consequence of D1).

---

## 3. Personas & Use Cases

- **The engineer (primary dev's profile).** Writes Slack standups, PR descriptions, code-review comments. Often has **elevated** windows open (admin terminal, elevated VS Code, Task Manager) — see Windows UIPI handling (§9.5).
- **The communicator.** Drafts customer emails and docs; cares about tone and grammar.

**Representative flow (v1, blind auto-inject):**
1. User is in Slack. Presses the hotkey (tap-to-toggle) — tray icon shows **recording**.
2. Says: *"hey team i'm gonna be like 10 mins late to standup, internet's acting up."*
3. Presses hotkey again to stop (or auto-stop on sustained silence).
4. HUD shows **processing**; ~2.5–5s later the refined text — *"Hey team, I'll be about 10 minutes late to standup — my internet is acting up."* — is pasted into the Slack composer. **The app does not press Enter.**

---

## 4. Success Metrics & SLOs

Release is gated on these (measured during dogfood). Latency is **re-baselined after the real network path exists** — the original "~2s" is optimistic; realistic p95 with two sequential network hops is 2.5–5s.

| Metric | Target (v1) | How measured |
|--------|-------------|--------------|
| End-to-end latency, 10s utterance | **p50 ≤ 2.5s, p95 ≤ 5s** | Per-stage timers (capture-finalize → upload → STT → LLM TTFT → LLM complete → paste). |
| Injection success (surrogate) | **≥ 99%** | Surrogate signals: focus-guard pass rate, paste-vs-typing fallback rate, clipboard-restore success, best-effort post-injection field-value read-back where the OS allows. *Explicitly a surrogate — there is no guaranteed read-back for a fire-and-forget paste.* |
| Crash-free sessions | **≥ 99.5%** | Opt-in crash reporting (pipeline stage + permission state, **never transcript content**). |
| Transcription accuracy | WER bar TBD on a fixed internal sample | Recorded fixture set. |

---

## 5. Functional Requirements

| ID | Requirement |
|----|-------------|
| FR-1 | **Global hotkey** registered system-wide; works regardless of focused app. Detect registration failure (collision) and offer rebind. Default avoids the common `Ctrl/Cmd+Shift+Space` collision (Spotlight/launchers/IME) — pick a lower-collision default and make it remappable. |
| FR-2 | **Activation model:** tap-to-toggle (start/stop) is the default for long-form; **hold-to-talk** is an opt-in quick-capture mode. `Esc` cancels and discards in either mode. (The global-shortcut plugin natively exposes both `Pressed` and `Released`, so hold is a real option, not a hack.) |
| FR-3 | **Audio capture** of the default input device while active, with **device-lifecycle handling**: on mid-recording device/route change (e.g. Bluetooth headset connects), re-attach to the new default or stop cleanly with a "mic changed — retry" message; never emit silent/garbled audio. Resample explicitly to the STT-required rate (16 kHz mono); never assume the device rate. Hard **60s recording cap** with a visible warning before cutoff. |
| FR-4 | **Recording affordance:** persistent tray-icon state change + an unobtrusive HUD; optional **auto-stop-on-silence** (configurable). |
| FR-5 | **Speech-to-text** via a swappable provider (§7). VAD/min-duration gate discards <300–500ms; empty/near-silent audio yields "no speech detected" and injects nothing. |
| FR-6 | **LLM refinement** via a user-selected **instruction profile**; output is *only* the final text. On refinement failure/timeout/malformed output, fall back to injecting the **raw transcript** flagged as raw (§7.4). |
| FR-7 | **Text injection** into the active field via **clipboard-paste primary, per-character typing fallback** (§8.3). Default behavior is **blind auto-inject** (D4). |
| FR-8 | **Focus safety:** capture the target window/field at recording start; immediately before injection, re-verify it is frontmost and identical. If focus changed, or the field is password/secure, **do not auto-inject** — copy to clipboard and show a "copied — paste manually" toast. |
| FR-9 | **Never synthesize Enter** (§6.5) — a deliberate safety non-goal. |
| FR-10 | **Concurrency policy — cancel-prior:** a new activation while a pipeline is in flight cancels and discards the prior result (never injects it) and starts fresh. No queueing, no out-of-order injection. |
| FR-11 | **Instruction profiles:** 2–3 named profiles (e.g. *Casual Slack*, *Formal email*, *Bulleted PR*) with a fast in-flow switcher. Per-app auto-mapping is roadmap. |
| FR-12 | **Optional preview:** per-profile toggle to show an editable preview before injection (off by default per D4). |
| FR-13 | **History (optional, off by default in dogfood):** last-N entries (transcript + refined text), **encrypted at rest** (OS-keychain-backed key), with max-age cap, one-click clear, and redaction of secrets-flagged content (§9.7). |
| FR-14 | **Settings UI:** provider API keys, profiles, hotkey/activation, model selection, history controls, data-handling disclosure. |
| FR-15 | **First-run onboarding wizard:** detect permission state, deep-link to OS panes, handle the macOS restart, run an end-to-end "test dictation," and present denied-permission fallback states + Windows SmartScreen guidance. |
| FR-16 | **Error surface:** a single consistent failure UI (HUD turns red + concise message + one recovery action) for every error class (§11). |

---

## 6. Interaction Model & UX

- **6.1 Activation.** Tap-to-toggle default; hold-to-talk opt-in; `Esc` cancels. Re-press = cancel-prior (FR-10).
- **6.2 Recording state** is communicated by a tray-icon state change *and* a HUD, plus optional auto-stop-on-silence and a pre-cap warning — mitigating the "forgot it was recording" failure of toggle mode.
- **6.3 Injection (default, blind).** Refined text is pasted automatically into the focused field after the focus-verify guard passes.
- **6.4 Optional preview** (per profile) shows the refined text in a small editable popover; one keypress confirms+pastes. Recommended for high-stakes profiles. A defined edit-rate metric can "graduate" a profile from preview to blind (§15).
- **6.5 Never auto-Enter — justified safety non-goal.** The app never presses Enter. Rationale: in terminals this prevents arbitrary command execution; in chat apps it prevents accidental send of unreviewed, non-deterministic LLM output. Users press Enter themselves. This is surfaced in onboarding so it reads as a deliberate guarantee, not a missing feature.
- **6.6 Latency masking (roadmap).** Optionally inject the raw transcript immediately and swap in refined text in place when refinement returns — a cheap alternative to streaming. Evaluated, not committed for v1.

---

## 7. AI Pipeline

Two swappable stages behind Rust traits (`SttProvider`, `Refiner`). Model IDs live in client config and can be updated via auto-update; optionally read from a tiny static remote **model manifest** (e.g. a GitHub-hosted JSON) so a retirement can be patched without a full release.

### 7.1 Speech-to-Text — OpenAI (single-provider stack)
- **Default:** OpenAI **`gpt-4o-mini-transcribe`** — fast/cheap tier; supports `stream=true` for completed recordings (ideal for our own push-to-talk/toggle turn detection — stream transcript deltas the instant recording stops, cutting *perceived* latency); supports a `prompt` for custom vocabulary (names, acronyms, product terms). Output `text`/`json`.
- **Quality-tier upgrade:** OpenAI **`gpt-4o-transcribe`** if accuracy on hard audio/accents is insufficient (also streams + supports prompts; slightly slower/pricier).
- **Not used:** `gpt-4o-transcribe-diarize` (diarization is for *multi-speaker* audio; we are single-speaker and it requires `chunking_strategy` + adds latency); `whisper-1` (older, no streaming); the **Realtime API / `gpt-realtime-whisper`** (only for continuous live-mic streaming — we do our own turn detection, so the file Transcription endpoint with `stream=true` is the correct, simpler path).
- **Roadmap:** local `faster-whisper`/`whisper.cpp` for an offline privacy mode (CPU-only laptops likely miss the SLOs — implies a separate slower SLO regime).
- *Lifecycle: confirm each model's retirement date on the OpenAI Models page before committing and at every release — lifecycles run in months. (The earlier automated-review claim that these STT models "retired 2026-06-01" was contradicted by the live docs and is withdrawn.)*

### 7.2 Refinement (LLM) — OpenAI nano (single-provider stack)
- **Default:** OpenAI **`gpt-4.1-nano`** — fastest tier and, crucially, **no reasoning tokens**. A grammar/tone/format rewrite is a pure transformation; a reasoning model would add unpredictable thinking latency for zero benefit on this task. Cheap ($0.10 in / $0.40 out per MTok).
- **Alternative to A/B test:** **`gpt-5-nano`** with **`reasoning_effort: "minimal"`** — newer, half the input cost ($0.05), marginally stronger instruction-following; test only if `gpt-4.1-nano` struggles on trickier formatting (e.g. "bulleted list for a PR"). Keep reasoning at `minimal` to protect latency.
- **Avoid:** `gpt-5.4-nano` (pricier output $1.25; more reasoning than this task needs).
- *Call non-streaming for short refinements; the `Refiner` trait keeps the model swappable, so Claude Haiku 4.5 / Sonnet 4.6 remain drop-in options if you ever want a second provider.*

> **Single-provider (resolved 2026-06-23):** STT (`gpt-4o-mini-transcribe`) and refinement (`gpt-4.1-nano`) are **both OpenAI → one API key per user**. This resolves the earlier two-key onboarding friction and means a single OpenAI ZDR/no-training + DPA agreement covers the whole pipeline (§9.6).

### 7.3 Refinement prompt design
- Role: a **silent text refiner**. Output **only** the refined text — no preamble, quotes, code fences, or commentary.
- Treat the transcript as **data, not instructions** — wrap it in XML-delimited blocks to prevent spoken-content prompt injection (e.g. the user literally saying "ignore previous instructions").
- Preserve meaning and tone; respond in the input's language; honor the active profile's style.

### 7.4 Fallback chain (resilience)
`VAD/min-duration gate` → `STT` → `empty/hallucination guard` (denylist of Whisper phantom phrases like "Thank you." / "Thanks for watching." + a `no_speech_prob` threshold; inject nothing on trip) → `refinement` (short configurable timeout) → on any refinement failure/timeout/malformed output, **inject the raw transcript flagged as raw**. The user always gets their words or a clear message — never silence.

### 7.5 Latency budget (per stage; gate optimization on the dominant stage)
`capture-finalize` + `upload` + `STT` + `LLM TTFT` + `LLM completion` + `paste`. Each gets a target; re-baseline p50/p95 after the real path exists. Measure actual TTFT in the spike rather than trusting a quoted figure.

---

## 8. Architecture & Tech Stack

### 8.1 Framework — **Tauri v2** (Rust core + system webview for settings)
Chosen over Electron: far smaller resident footprint for an always-on tray app, native tray/background, first-party signed-updater plugin and build-matrix CI, smaller attack surface (no bundled Chromium), strong Windows/WebView2 story matching the Windows-first dev. (Electron's RobotJS is unmaintained; Nut.js would be its only viable injection lib.)

### 8.2 Core components (Rust)
- **State machine** (the single owner of the loop): `Idle → Recording → Processing → Injecting → Idle`, in Tauri managed state (tokio task + channels), enforcing legal transitions, `Esc`-cancel, timeouts, and the cancel-prior policy (FR-10).
- **Traits (swappable):** `AudioCapture`, `SttProvider`, `Refiner`, `Injector`. Each has a test double (echo provider, recorded-fixture STT).
- **Audio:** `cpal` on Windows; **`getUserMedia` in a hidden webview on macOS** (see §9.3 — these are *not* interchangeable). The capture stack is part of the trait contract and the spike's validation matrix.
- **Injection:** see §8.3.
- **Secrets:** `keyring` crate → Windows Credential Manager / macOS Keychain. **Never** plaintext config.

### 8.3 Text injection — **clipboard-paste primary, typing fallback**
The single most important architectural correction from the draft: **do not type text character-by-character as the primary path.** Per-character injection (Enigo) is the least reliable part of any dictation app — it breaks on long/multi-paragraph/Unicode/emoji text, focus loss, IME/dead-key composition, and has known macOS modifier-composition bugs.

**Primary path:**
1. Snapshot the **full** clipboard (all formats — text, image, RTF, files), not just text.
2. Set clipboard to the refined text (mark transient/concealed — *best-effort only*, see §9.4).
3. Synthesize `Ctrl+V` (Win) / `Cmd+V` (mac) via explicit key-down/key-up.
4. **Restore the full original clipboard** via a guaranteed-restore routine that runs even on error/panic (RAII / `Drop` guard).

**Typing fallback (per-character)** is used for: paste-hostile fields, **and deliberately for sensitivity-flagged fields** (avoids clipboard transit entirely). Before pasting, check for an active IME/composition buffer and commit/abort it first. Pin Enigo to current `0.6.x`; validate the macOS modifier path in the spike.

---

## 9. Security, Privacy & Permissions

### 9.1 macOS permissions — **correct primitives**
- **Accessibility / Input Monitoring** required for synthetic input and reading focus.
- **Live check:** `CGEvent.tapCreate(.listenOnly)` — creation succeeds only if the grant is live *now*. Do **not** rely on `AXIsProcessTrusted()` for the runtime check — it reads a per-process cache that goes stale across OS/app updates for exactly this kind of long-running tray app (and can return `true` while event-tap creation fails). Use `AXIsProcessTrustedWithOptions` only for the *initial* prompt.
- **Re-verify the permission live immediately before every injection and after every auto-update**; surface remediation rather than failing silently.

### 9.2 macOS grant persistence across updates
TCC keys on code signature **AND** bundle id **AND** resolved on-disk path. Tauri-style versioned binaries land at a new path each update and look like a brand-new app, silently dropping the grant. Mitigation: (a) freeze a reverse-DNS **bundle id** forever; (b) ship a **stable launcher binary at a fixed install path** that `exec`s the versioned binary, so TCC grants the stable path once; (c) keep **one Developer ID** across all releases.

### 9.3 Microphone (macOS signed-build trap)
`cpal` on a **signed** macOS build can silently capture **no audio** — the TCC mic prompt never appears — even with `NSMicrophoneUsageDescription` set, unless the `com.apple.security.device.audio-input` entitlement is present. Mitigation: include the audio-input entitlement, trigger the mic TCC prompt once via `getUserMedia` at onboarding, and **capture via `getUserMedia` on macOS** (cpal on Windows). Validate end-to-end on a **notarized** `.dmg` — this fails only in the distributed build, not in dev.

### 9.4 Clipboard leak (honest framing)
The NSPasteboard/Windows transient/concealed hint is **advisory**. Most clipboard managers (Ditto, Maccy) and OS sync (Windows Cloud Clipboard, macOS Universal Clipboard/Handoff) ignore it and **will** capture the refined text during the paste window. Therefore: paste-transit is a **residual leak**, mitigated by routing **sensitivity-flagged fields through the typing fallback** (§8.3) so they never touch the clipboard. Do not present the hint as a guarantee.

### 9.5 Windows injection / elevation (UIPI)
Windows UIPI silently blocks synthetic input into higher-integrity (elevated) processes unless the app is itself elevated. The engineer persona runs elevated terminals/VS Code/Task Manager. Behavior: detect when the foreground window belongs to a higher-integrity process (`GetWindowThreadProcessId` + token integrity level); when injection would be UIPI-blocked, **do not fail silently** — copy to clipboard + show "copied — paste manually (elevated window)." **Default: clipboard fallback, no elevation** (avoids the security/packaging burden of `uiAccess`). See §15 to confirm.

### 9.6 Data handling under per-user keys + ZDR (D1 + D3)
- **Audio is held in memory only and never persisted.**
- **ZDR + no-training + DPA** are a launch requirement, configured at the **provider-account level** for both STT and refinement endpoints. Justification rests on **data classification and contractual posture** (internal Slack/email/PII is the highest-risk payload) — *not* on any court order. *(Correction: the OpenAI/NYT preservation order ended ~Sept 2025; OpenAI returned to standard ~30-day API deletion. Do not cite it.)*
- **Per-user-keys limitation (be explicit to stakeholders):** with no proxy, there is **no central enforcement** of no-body-logging, **no central rotation/revocation**, and **no central cost cap**. ZDR/no-training must be set in each provider workspace; recommend issuing keys from **org-managed provider workspaces** with ZDR enabled, not personal accounts. Offboarding = revoke the user's keys in the provider console + the app clears its keychain entries on uninstall.
- **Logging/telemetry:** scrub transcripts/audio/prompts from all logs and crash reports by default.
- **In-app data-handling disclosure** shown once at onboarding (everything spoken is sent to third-party APIs, even under ZDR), with `docs/data-handling.md` linked from Settings.

### 9.7 Local history at rest
If history is enabled (off by default in dogfood): encrypt at rest with an OS-keychain-backed key; cap by count and age; one-click clear; never store secrets-flagged content. On a departing employee's laptop this would otherwise be unmanaged sensitive data.

---

## 10. Settings & Configuration
- **Provider keys** (Groq STT key, Anthropic refinement key — or single-provider mode), stored in OS keychain.
- **Profiles:** 2–3 named instruction profiles + in-flow switcher; per-profile preview toggle.
- **Activation:** hotkey (remappable, collision-detected), tap-vs-hold, auto-stop-on-silence, recording cap warning.
- **Model selection:** STT + refinement tier (Haiku default / Sonnet / Opus).
- **History controls:** enable/disable, retention, clear.
- **Versioned config** (`schema_version` + migrations) in the OS app-config dir.

---

## 11. Error Handling

Every failure resolves to one consistent surface (red HUD + concise message + one recovery action). The user always gets their words or a clear message — never silence.

| Class | Behavior |
|-------|----------|
| No mic / mic permission denied | Block + deep-link to OS pane (onboarding remediation). |
| Mic changed mid-recording | Re-attach or "mic changed — retry." |
| No network / provider down | "Couldn't reach the service — retry"; never partial inject. |
| Bad/missing API key | Prompt to re-enter key in Settings. |
| Rate limited (429) / 5xx | Exponential backoff + retry in client; surface if exhausted. |
| Silent/empty audio | "No speech detected" — inject nothing. |
| STT hallucination (phantom phrase) | Suppressed by denylist + `no_speech_prob` guard. |
| Refinement fail/timeout/malformed | Inject **raw transcript** flagged as raw. |
| Focus changed / secure field | Don't auto-inject — clipboard + "paste manually" toast. |
| UIPI-blocked (elevated target) | Clipboard + "paste manually (elevated window)." |

---

## 12. Delivery, Packaging, Signing & Updates

- **Windows signing:** **Azure Trusted/Artifact Signing** (CI-friendly, no HSM token, managed renewal). Note: EV certs no longer buy instant SmartScreen reputation (removed 2024) — reputation accrues from download volume; public code-signing cert validity is being capped to ~458–460 days (CA/Browser Forum CSC-31, effective ~2026-03-01) — prefer managed renewal.
- **macOS signing/notarization:** **one Developer ID** + hardened runtime + notarization on the **first** build (not at the end). Apple Developer **org enrollment** (D-U-N-S) is a multi-day-to-week lead item — **start week 0.** Treat `notarytool` round-trips as **in the critical path** of every "fast" update.
- **Distribution + updates:** **Tauri updater + GitHub Releases**, artifacts **minisign-signed** (back up the minisign private key in a team secret manager — losing it means never shipping another update). Add **staged rollout (canary subset)** and a **rollback/kill-switch** channel — a bad update (TCC regression, crash loop) across the fleet is worse than slow patching. After every auto-update, **re-verify macOS permissions live** and prompt to remediate if regressed. (If the org runs Jamf/Intune, managed distribution + TCC PPPC pre-grants is even better — see §15.)
- **Crash reporting:** opt-in (e.g. Sentry Rust+JS), capturing pipeline-stage + permission state, **never transcript content**.

---

## 13. Phased Roadmap

> The original "skip the Python PoC" intent is honored — **there is no throwaway Python.** Phase 0 is a *walking skeleton in the real Tauri/Rust stack*; it de-risks the two subsystems that actually sink these projects (injection into real apps; signed-build audio/permissions) without throwaway work.

- **Phase 0 — De-risking walking-skeleton spike (real stack).**
  - **Windows (1–2 weeks, starts now):** hotkey → capture (cpal) → Groq STT → **clipboard-paste a transcribed string into real Slack, Outlook/Gmail, and VS Code.** Prove injection + audio + the full latency path. Measure real TTFT.
  - **macOS (gated, starts week 0 in parallel):** Apple org enrollment + a borrowed-Mac/macOS-CI path; then the same skeleton validated on a **notarized** build (getUserMedia capture, TCC persistence). *Its success criterion can't be met in week 1 if enrollment hasn't cleared — hence Windows-first.*
- **Phase 1 — Windows internal v1.** State machine, refinement profiles, blind auto-inject + focus guard, onboarding wizard, error surface, settings, keychain, history (off by default), Azure signing, Tauri updater + staged rollout/rollback, opt-in crash reporting. Gate on the §4 SLOs.
- **Phase 2 — macOS fast-follow.** Once enrollment + macOS CI are stood up: getUserMedia capture, TCC live-check + stable-launcher persistence, notarized distribution.
- **Phase 3 — Hardening & roadmap.** Per-app profile auto-mapping; latency-masking (raw-then-swap); optional local/offline STT privacy mode; multilingual (if confirmed).

---

## 14. Proposed Repository / Project Structure

```
voicetype-ai/
├─ Cargo.toml                      # Rust workspace
├─ package.json                    # frontend (settings UI) deps
├─ README.md
├─ docs/
│  ├─ data-handling.md             # the disclosure surfaced in Settings
│  ├─ architecture.md
│  └─ runbooks/
│     ├─ offboarding.md            # revoke keys, wipe history
│     └─ release-and-rollback.md   # minisign key, staged rollout, kill-switch
├─ crates/
│  ├─ core/                        # state machine, orchestration, traits
│  │  └─ src/{state.rs,traits.rs,pipeline.rs,error.rs}
│  ├─ audio/                       # AudioCapture impls (cpal Win / webview-bridge mac)
│  ├─ stt/                         # SttProvider: groq.rs, openai.rs, (local/ later)
│  ├─ refine/                      # Refiner: anthropic.rs, prompts/, profiles
│  ├─ inject/                      # Injector: clipboard.rs, typing.rs, focus_guard.rs
│  ├─ hotkey/                      # global-shortcut wiring, activation modes
│  ├─ secrets/                     # keyring wrapper
│  ├─ permissions/                 # macOS CGEvent check + persistence; Win UIPI/integrity
│  └─ config/                      # versioned settings + migrations
├─ src-tauri/                      # Tauri app shell, tray, updater, IPC commands
│  ├─ src/main.rs
│  ├─ tauri.conf.json
│  └─ entitlements.mac.plist       # incl. com.apple.security.device.audio-input
├─ ui/                             # settings webview (framework TBD)
│  └─ src/{onboarding/,settings/,hud/}
├─ tests/
│  ├─ fixtures/                    # recorded audio + expected transcripts
│  └─ e2e/                         # OS injection smoke tests vs a harness app
└─ .github/workflows/
   ├─ ci.yml                       # build matrix (win + mac), unit/integration
   └─ release.yml                  # sign (Azure / Developer ID), notarize, minisign, publish
```

**Testing pyramid:** unit (mocked STT/LLM + echo provider) → integration (recorded fixtures) → OS e2e injection smoke tests asserting the received text in a harness app.

---

## 15. Open Items & Decisions Log

Defaults are set so the build is not blocked; each needs an owner to confirm.

| Item | Default in this PRD | Needs |
|------|---------------------|-------|
| ZDR/no-training + DPA signed (both STT & LLM endpoints) | Required before launch | Procurement/IT to obtain; keys issued from org-managed workspaces. |
| Data-classification ceiling (may users dictate credentials/regulated PII?) | Assume no regulated PII in v1 | IT/security to confirm; if yes, on-device STT may be required (slower SLO). |
| Data residency / GDPR (any EU/UK users?) | Assume US-based | Confirm geography; if EU/UK, choose regional endpoints / transfer basis before launch. |
| Provider stack | **Resolved: single-provider OpenAI** (`gpt-4o-mini-transcribe` + `gpt-4.1-nano`), one key/user | — |
| Windows elevation policy | Clipboard fallback for elevated targets, no `uiAccess` | Confirm acceptable vs requesting elevation. |
| Preview→blind "graduation" metric | Blind default (D4); preview optional | If preview is used, define the edit-rate gate (e.g. <X% edits over N injections). |
| Endpoint/MDM stack (AV/EDR, Jamf/Intune) | Assume none managed | Confirm; MDM eases distribution, TCC pre-grants, offboarding wipe; pre-clear AV allow-listing for an input+mic binary. |
| Latency ceiling | Aspirational ~2.5–5s p95 | Confirm hard vs soft; informs raw-then-swap masking and streaming. |
| Language mix | English-first | Confirm; multilingual requires STT benchmarking on actual languages. |
| Hotkey default | Low-collision, remappable | Pick final default during spike. |

---

## 16. Cost Model (back-of-envelope — confirm current rates)

Assumptions: 25 users × ~15 min dictation/day × 22 workdays ≈ **137 hrs/month** (~8,200 min) audio; ~3k input + 3k output tokens/user/day ≈ **~1.65M in + 1.65M out tokens/month**.

- **STT (`gpt-4o-mini-transcribe`):** ~8,200 audio-minutes/month — at typical mini-transcribe rates roughly **$10–30/month** (confirm the current per-minute/token audio rate on the OpenAI Pricing page).
- **Refinement (`gpt-4.1-nano`, $0.10/$0.40 per MTok):** ≈ **<$1/month** (~$0.17 in + $0.66 out).
- **Team total:** **~$15–30/month** — a genuine rounding error. The `gpt-4o-transcribe` quality tier would raise the STT line ~2–3×; still small.
- Add per-request timeouts, exponential backoff on 429/5xx, and the 60s recording cap regardless.

---

## 17. Top Risks & Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Injection unreliable in real apps (the payoff moment) | Critical | Clipboard-paste primary + typing fallback; focus guard; validate in Phase 0 spike against real Slack/Mail/VS Code. |
| macOS signed-build mic capture silently fails | Critical | audio-input entitlement + getUserMedia capture; validate on notarized `.dmg`. |
| macOS TCC grant lost on auto-update | Critical | Stable launcher path + frozen bundle id + one Developer ID; live re-check post-update. |
| Sensitive content leaks (cloud or clipboard) | Critical | ZDR/no-training + DPA; audio memory-only; typing fallback for sensitive fields; logs scrubbed. |
| Per-user keys: no central rotation/revocation/cost cap | High | Org-managed provider workspaces; offboarding runbook; per-user spend awareness; accept as a known D1 trade-off. |
| Bad auto-update bricks the fleet | High | Staged rollout + rollback/kill-switch; minisign key backed up. |
| macOS schedule blocked by Apple enrollment | High | Windows-first; start enrollment + CI week 0. |
| Concurrency/late-injection into wrong field | High | Cancel-prior policy + focus re-verify before inject. |

---

*End of PRD v1.0.*
