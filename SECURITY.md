# Security Policy

VoiceType AI is a desktop voice-dictation app that does two security-sensitive
things by design: it **synthesizes keystrokes / pastes into other applications**,
and it **stores an OpenAI API key in your operating system's keychain**. We take
reports about either area seriously and appreciate responsible disclosure.

This project is maintained by [Devaxl](https://devaxl.com).

## Supported versions

VoiceType AI is currently at a working **v0** — pre-release, not yet packaged or
code-signed. Security fixes land on the `main` branch.

| Version               | Supported          |
| --------------------- | ------------------ |
| `main` (latest)       | :white_check_mark: |
| Older commits / forks | :x:                |

Once we cut tagged releases, this table will be updated to list the supported
release lines. Until then, please test against the latest `main`.

## Reporting a vulnerability

**Please report security issues privately. Do not open a public GitHub issue,
pull request, or discussion for a vulnerability.**

Preferred channels:

1. **GitHub private vulnerability reporting** — go to the
   [repository's Security tab](https://github.com/devaxl/VoiceType-AI/security)
   and use **"Report a vulnerability"**. This is the fastest way to reach the
   maintainers.
2. **Email** — contact Devaxl via [https://devaxl.com](https://devaxl.com) and
   mark the message clearly as a VoiceType AI security report.

Please include, as far as you can:

- A description of the issue and its security impact.
- Step-by-step reproduction (and a proof-of-concept if you have one).
- Affected commit / version, your OS and version, and the Rust toolchain
  (MSVC or GNU) if relevant.
- Any suggested remediation.

**Do not include real secrets** — your OpenAI API key, real audio, or real
transcript content — in a report. Redact anything sensitive.

### What to expect

This is a small, community-maintained project, so please allow reasonable time
for a response. We aim to acknowledge a report within a few business days, work
with you on a fix, and credit you in the release notes if you'd like. Please give
us a reasonable opportunity to address the issue before any public disclosure
(coordinated disclosure).

## Scope

Because of what the app does, the following are **in scope** and especially
valuable to report:

- **Keystroke / text injection** — ways to make the app paste or type into the
  **wrong window or field**, bypass the focus-verify guard, defeat the
  secure/password-field typing bypass, or otherwise inject without the
  appropriate guards (e.g. inducing it to synthesize **Enter**, which it must
  never do).
- **API key / secrets handling** — any path that exposes the user's OpenAI API
  key from the OS keychain, writes it to disk or logs in plaintext, or leaks it
  to an unintended destination.
- **Clipboard handling** — flaws in the save/restore routine that leak data or
  fail to restore the user's original clipboard, beyond the already-documented
  advisory-only limits of clipboard managers and OS clipboard sync (see PRD §9.4
  and `docs/data-handling.md`).
- **Audio / transcript data exposure** — anything that causes audio (which is
  meant to stay in memory only) or transcripts to be written to disk, logged, or
  sent somewhere other than the user's configured OpenAI endpoint.
- **Local privilege / integrity issues** in the desktop binary, IPC commands, or
  configuration handling.
- **Supply-chain / build** concerns affecting the produced binaries.

### Out of scope / known limitations

These are documented design facts or known v0 gaps, not vulnerabilities — please
don't report them as such (they're covered in the README and PRD):

- **No code signing yet** — Windows SmartScreen may warn on first run and macOS
  builds are unsigned (users right-click → Open to bypass Gatekeeper). Signing is
  a planned distribution step.
- **Data is sent to OpenAI under your own key.** Audio and transcripts go to the
  OpenAI API by design. For sensitive content we recommend an OpenAI workspace
  with a Zero-Data-Retention / no-training agreement. Risks inherent to using a
  third-party cloud API under your key are out of scope.
- **Clipboard manager / OS clipboard-sync capture during the paste window.** The
  transient/concealed clipboard hint is advisory; tools like Ditto/Maccy and
  Windows Cloud Clipboard / macOS Universal Clipboard may capture the pasted
  text. This is a documented residual limitation (mitigated by routing sensitive
  fields through the typing fallback).
- **Secure-field detection gaps** — only classic Win32 password fields are
  currently detected; browser/Electron password inputs and Windows
  elevated-window (UIPI) targets are known, documented v0 gaps. Concrete bypasses
  *beyond* these documented gaps are still worth reporting.
- General OS, WebView2, or third-party dependency vulnerabilities that are not
  specific to VoiceType AI (please report those upstream).

Thank you for helping keep VoiceType AI and its users safe.
