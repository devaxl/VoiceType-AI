<div align="center">

# 🎙️ VoiceType AI

### Press a hotkey, speak, and watch AI-refined text appear in any app — using your own OpenAI key.

A private, bring-your-own-key voice dictation tool for Windows (macOS via CI). Open source. Built with Tauri v2 + Rust.

[![License: MIT](https://img.shields.io/badge/License-MIT-22c55e.svg?style=flat-square)](LICENSE)
[![Latest Release](https://img.shields.io/github/v/release/devaxl/VoiceType-AI?style=flat-square&color=6366f1)](https://github.com/devaxl/VoiceType-AI/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/devaxl/VoiceType-AI/ci.yml?style=flat-square&label=build)](https://github.com/devaxl/VoiceType-AI/actions)
[![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20macOS-0ea5e9?style=flat-square)](#-download--install)
[![Stars](https://img.shields.io/github/stars/devaxl/VoiceType-AI?style=flat-square&color=eab308)](https://github.com/devaxl/VoiceType-AI/stargazers)
[![Built by Devaxl](https://img.shields.io/badge/built%20by-Devaxl-ec4899?style=flat-square)](https://devaxl.com)

[Download](#-download--install) · [Quick Start](#-quick-start-first-run) · [How It Works](#-how-it-works) · [Privacy](#-privacy--data-handling) · [Roadmap](#-roadmap)

</div>

---

## What is VoiceType AI?

**VoiceType AI** turns your voice into polished, ready-to-paste text in *any* application — Slack, Outlook, VS Code, your browser, a terminal, anywhere you can type. Press a global hotkey, dictate naturally, press it again, and a moment later the refined text is injected straight into the focused field. No app switching. No copy-paste dance.

It's a **bring-your-own-key** tool: you supply your own OpenAI API key, it lives in your operating system's keychain (never in plaintext), and your audio is held in memory only — never written to disk. Speech is transcribed by OpenAI's `gpt-4o-mini-transcribe`, then cleaned up for grammar, tone, and formatting by `gpt-4.1-nano` using a refinement profile *you* control.

If you've wanted a private, hackable, open-source alternative to commercial dictation apps like Wispr Flow — one where you own the keys, the data path, and the code — this is it.

> **Status: working v0.** Built on a real production stack (Tauri v2 + Rust core), not a throwaway prototype. The full core loop, background/tray mode, safe-inject guards, a rebindable hotkey, refinement profiles, and persisted settings all work today. It is **not yet code-signed or packaged with an installer**, and macOS builds come from CI rather than a native dev environment (see [Roadmap](#-roadmap)).

<!-- 📽️ DEMO GIF / SCREENSHOT GOES HERE -->
<!-- Drop a short screen recording (hotkey → speak → refined text appears in Slack) at docs/demo.gif and reference it below: -->
<!-- <div align="center"><img src="docs/demo.gif" alt="VoiceType AI in action" width="720"></div> -->

---

## ✨ Features

Everything below is **implemented and working today** in v0.

### 🎤 Capture & Dictate
- ⌨️ **Global hotkey, works anywhere** — default `Alt+Shift+D`, tap to start / tap to stop. Active even when the window is hidden.
- 🔁 **Rebindable shortcut** — change the global hotkey right from the settings UI.
- 🎙️ **Low-latency mic capture** — native audio capture via `cpal` on a dedicated thread.

### 🧠 Transcribe & Refine
- 🗣️ **OpenAI speech-to-text** — fast, accurate transcription with `gpt-4o-mini-transcribe`.
- ✍️ **AI refinement** — `gpt-4.1-nano` cleans up grammar, tone, and formatting so you paste finished text, not a raw transcript.
- 📝 **Named refinement profiles** — switch between **General / Casual / Formal / Bulleted**, each fully editable, with add/delete and a quick switcher. Your active profile persists.
- 📚 **Custom vocabulary** — feed the transcriber your jargon, product names, and acronyms so it spells your terms correctly.

### 🛡️ Safe, Reliable Injection
- 🎯 **Clipboard-paste injection** into the active field, with a per-character **typing fallback** for paste-hostile inputs.
- 🔍 **Focus-verify guard** — confirms the original field is still frontmost before injecting, so text never lands in the wrong window.
- 📋 **Full clipboard save & restore** — your existing clipboard (text + image) is snapshotted and restored, even on error.
- 🔒 **Secure-field bypass** — password/secure fields route through typing to avoid clipboard transit.
- 🧹 **Hallucination & empty-audio filter** — silent or phantom-phrase recordings inject nothing instead of garbage.
- ⛔ **Cancel-prior concurrency** — re-triggering while a pipeline is in flight discards the stale result and starts fresh.

### 🪟 Stays Out of Your Way
- 📌 **Background / system-tray mode** — closing the window hides to the tray; the hotkey keeps working.
- 💬 **Floating status HUD** — a click-through pill shows recording / processing / success / error, so you get feedback even when the main window is hidden.
- 🌐 **Network resilience** — STT and refine calls retry with backoff on transient errors, with request timeouts; a failed paste retries, then falls back to clipboard.
- 💾 **Persisted settings** — profiles, vocabulary, and hotkey are saved to disk (`%APPDATA%\com.devaxl.voicetype\config.json`).

---

## 📥 Download & Install

> **Heads up:** VoiceType AI is **not yet code-signed.** The app is safe and fully open source — you can read every line here — but unsigned binaries trigger OS warnings on first launch. This is expected, and code signing is on the [roadmap](#-roadmap).

1. Head to the [**Releases**](https://github.com/devaxl/VoiceType-AI/releases) page.
2. Download the latest build for your platform.

### Windows
- Run the installer/executable.
- **Windows SmartScreen** may show a *"Windows protected your PC"* warning because the build isn't signed yet. Click **More info → Run anyway** to proceed.

### macOS (built via CI)
- macOS builds are produced by CI and are **unsigned / not notarized**.
- macOS Gatekeeper will block the first launch. **Right-click (or Control-click) the app → Open**, then confirm in the dialog to run it.

#### macOS troubleshooting

**The app won't open.** Because the build is unsigned, macOS may refuse to launch it on the first try. Either **right-click the app → Open** (then confirm), or open **System Settings → Privacy & Security**, scroll to the bottom, and click **Open Anyway** next to the VoiceType AI message.

**Microphone or Accessibility permission is stuck** — e.g. *"speech not detected"* even after granting mic access, VoiceType AI missing from the permission list, or the hotkey not injecting text. Unsigned/ad-hoc builds can get a fresh code identity per install, so macOS sometimes holds on to stale permission state. Give the app a stable ad-hoc signature and reset its privacy grants, then relaunch and re-approve when prompted:

```bash
# Give the installed app a stable ad-hoc code signature
codesign --force --deep --sign - "/Applications/VoiceType AI.app"

# Reset just Accessibility, or reset ALL of the app's privacy grants
sudo tccutil reset Accessibility com.devaxl.voicetype
sudo tccutil reset All com.devaxl.voicetype
```

After running these, reopen VoiceType AI and approve the **Microphone** (and **Accessibility**, if asked) prompts — dictation should then work.

> Prefer to build it yourself? See [Build from Source](#-build-from-source) — it takes one `npm install` and one command.

---

## 🚀 Quick Start (First Run)

Get from zero to dictating in under a minute:

1. **Launch VoiceType AI.** The settings window opens.
2. **Paste your OpenAI API key** and save it. (Grab one from [platform.openai.com/api-keys](https://platform.openai.com/api-keys).) It's stored securely in your OS keychain — never in plaintext.
3. **Pick a refinement profile** — try **Casual** for Slack, **Formal** for email, **Bulleted** for PR descriptions.
4. **Put your cursor in any text field** — Slack, Notepad, VS Code, your browser.
5. **Press `Alt+Shift+D`**, speak naturally, then **press it again** to stop.
6. Watch the refined text appear right where your cursor is. ✨

> 💡 VoiceType AI never presses Enter for you — your dictated text is pasted, and *you* decide when to send. That's a deliberate safety guarantee, not a missing feature.

---

## ⚙️ Configuration

Everything is editable from the settings window:

| Setting | What it does |
|---|---|
| 🔑 **OpenAI API key** | Stored in the OS keychain (Windows Credential Manager / macOS Keychain) — **never** written to plaintext config. |
| 📝 **Refinement profiles** | Named instruction styles (General / Casual / Formal / Bulleted). Edit the wording, add your own, delete ones you don't use, and switch the active profile on the fly. |
| 📚 **Custom vocabulary** | A list of names, acronyms, and product terms passed to the transcriber so it gets your spelling right. |
| ⌨️ **Custom hotkey** | Rebind the global shortcut to whatever fits your muscle memory. |

Settings persist to `%APPDATA%\com.devaxl.voicetype\config.json` (your API key stays in the keychain, separately).

---

## 🔧 How It Works

The pipeline, end to end:

```
  Global hotkey  ──►  🎤 Mic capture (cpal)  ──►  🗣️ OpenAI STT (gpt-4o-mini-transcribe)
       (tap)                                                       │
                                                                   ▼
   ⌨️ Inject into focused field  ◄──  🛡️ Safe-inject guards  ◄──  ✍️ AI refine (gpt-4.1-nano + your profile)
   (clipboard-paste, typing fallback)   (focus-verify, secure-field, etc.)
```

**Architecture note.** VoiceType AI is a **Tauri v2** app. The latency-sensitive core — hotkey handling, audio capture, the state machine (`Idle → Recording → Processing → Injecting`), injection, and secrets — is written in **Rust**. The settings UI is a small **React + Vite** webview. The Rust modules map cleanly to the pipeline stages: `hotkey.rs`, `audio.rs`, `stt.rs`, `refine.rs`, `inject.rs`, `winfocus.rs`, `pipeline.rs`, `secrets.rs`, and `tray.rs`, all coordinated by a single state machine in `state.rs`.

---

## 🔐 Privacy & Data Handling

Privacy is a first-class design goal, and we're honest about the trade-offs:

- 🧠 **Audio stays in memory.** Your recordings are held in RAM only and **never written to disk**.
- 🔑 **Your key, your account.** Audio and transcripts are sent to **OpenAI under your own API key** — there's no Devaxl server in the middle, no central proxy, and no telemetry on your content.
- 🗝️ **Secrets stay in the keychain.** Your API key lives in the OS keychain, never in plaintext config.
- 🏢 **For sensitive content, use a ZDR workspace.** Because requests go to OpenAI, we recommend issuing your key from an **OpenAI workspace with a Zero-Data-Retention (ZDR) / no-training agreement** for any confidential material. This is configured at your provider account level.
- ⚠️ **Be aware:** anything you dictate is sent to a third-party API (OpenAI), even under ZDR. VoiceType AI makes the data path explicit so you can make an informed choice.

See [`docs/data-handling.md`](docs/data-handling.md) for the full disclosure.

---

## 🏗️ Build from Source

### Prerequisites
- [**Rust**](https://rustup.rs) (stable). On Windows, the **MSVC** toolchain (VS C++ Build Tools) is recommended; the **GNU** toolchain also works but needs a clone path without spaces (see [CONTRIBUTING.md](CONTRIBUTING.md)).
- [**Node.js**](https://nodejs.org) LTS.
- **WebView2** — preinstalled on Windows 11.

### Run in development
```bash
npm install
npm run tauri dev
```
Then paste your OpenAI API key in the settings window and press `Alt+Shift+D` to try it.

### Build installers
```bash
npm run tauri build
```
Or let **GitHub Actions** build cross-platform artifacts automatically when you push a version tag.

---

## 🗺️ Roadmap

VoiceType AI v0 works today; here's where it's headed:

- 🤖 **Anthropic / Claude support** — a second refinement provider behind the existing swappable trait.
- ⚡ **Streaming STT** — stream transcript deltas the instant you stop recording, cutting perceived latency.
- 🎯 **Per-app profile auto-switching** — automatically pick the right refinement profile based on the foreground app.
- ✅ **Code signing (Windows)** — eliminate the SmartScreen warning.
- 🍎 **macOS hardening** — getUserMedia capture, TCC permission live-checks, notarized distribution, and a stable launcher path for grant persistence.

---

## 🤝 Contributing

Contributions are very welcome — issues, feature ideas, and PRs alike. Please read [**CONTRIBUTING.md**](CONTRIBUTING.md) to get started, then open an issue or a pull request on the [repo](https://github.com/devaxl/VoiceType-AI).

If VoiceType AI is useful to you, the easiest way to help is to ⭐ **star the repo** — it genuinely helps others discover the project.

---

## 🧰 Tech Stack

| Layer | Technology |
|---|---|
| **App framework** | [Tauri v2](https://tauri.app) (Rust core + system webview) |
| **Core language** | Rust (stable) |
| **Settings UI** | React + Vite |
| **Audio capture** | [`cpal`](https://crates.io/crates/cpal) |
| **Injection** | [`enigo`](https://crates.io/crates/enigo) (typing) + [`arboard`](https://crates.io/crates/arboard) (clipboard) |
| **Secrets** | [`keyring`](https://crates.io/crates/keyring) → OS keychain |
| **Speech-to-text** | OpenAI `gpt-4o-mini-transcribe` |
| **Refinement** | OpenAI `gpt-4.1-nano` |
| **CI / release** | GitHub Actions |

*Keywords: voice dictation, speech-to-text, AI dictation, OpenAI, Whisper, Tauri, Rust, React, Windows, macOS, open source, productivity, Wispr Flow alternative.*

---

## 📄 License

Released under the [**MIT License**](LICENSE). Use it, fork it, ship it.

---

<div align="center">

## 💜 Built by Devaxl

**VoiceType AI** is built and maintained by **[Devaxl](https://devaxl.com)** — a team that loves crafting fast, private, well-engineered software and sharing it with the community.

We believe great developer tools should be open, hackable, and respect your data. If that resonates, come say hello.

### 🌐 [**devaxl.com**](https://devaxl.com)

---

While you're here, check out **[nrtur](https://nrtur.io)** — another project from the people behind Devaxl. Pay it a visit at **[nrtur.io](https://nrtur.io)**.

---

If VoiceType AI saves you some keystrokes, please ⭐ **[star the repo](https://github.com/devaxl/VoiceType-AI)** and share it. Thank you! 🙏

</div>
