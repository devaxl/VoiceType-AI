import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Status = "idle" | "recording" | "processing";
type Provider = "openai" | "anthropic" | "groq";

interface Profile {
  name: string;
  prompt: string;
}

interface Config {
  profiles: Profile[];
  active_profile: number;
  vocabulary: string;
  stt_provider: Provider;
  stt_model: string;
  refine_provider: Provider;
  refine_model: string;
  hotkey: string;
}

const STATUS_LABEL: Record<Status, string> = {
  idle: "Idle",
  recording: "● Recording…",
  processing: "… Processing",
};

// Curated model menus. Voice (STT) offers OpenAI + Groq — Anthropic has no transcription API, so
// it appears only under refinement.
const STT_CATALOG: { provider: Provider; label: string; models: string[] }[] = [
  { provider: "groq", label: "Groq — free tier", models: ["whisper-large-v3-turbo", "whisper-large-v3"] },
  { provider: "openai", label: "OpenAI", models: ["gpt-4o-mini-transcribe", "gpt-4o-transcribe", "whisper-1"] },
];
const REFINE_CATALOG: { provider: Provider; label: string; models: string[] }[] = [
  { provider: "anthropic", label: "Anthropic (Claude)", models: ["claude-haiku-4-5", "claude-sonnet-5"] },
  { provider: "openai", label: "OpenAI", models: ["gpt-4.1-nano", "gpt-4o-mini"] },
];

const PROVIDER_LABEL: Record<Provider, string> = {
  openai: "OpenAI",
  anthropic: "Anthropic",
  groq: "Groq",
};
const KEY_PLACEHOLDER: Record<Provider, string> = {
  openai: "sk-…",
  anthropic: "sk-ant-…",
  groq: "gsk_…",
};
const KEY_HELP: Record<Provider, string> = {
  openai: "platform.openai.com/api-keys",
  anthropic: "console.anthropic.com → API keys",
  groq: "console.groq.com/keys — free, no card",
};

const MODEL_LABEL: Record<string, string> = {
  "whisper-large-v3-turbo": "Whisper v3 Turbo — fast, free",
  "whisper-large-v3": "Whisper v3 — free",
  "gpt-4o-mini-transcribe": "gpt-4o-mini-transcribe",
  "gpt-4o-transcribe": "gpt-4o-transcribe — higher accuracy",
  "whisper-1": "whisper-1 — legacy",
  "claude-haiku-4-5": "Claude Haiku 4.5 — fast, cheap",
  "claude-sonnet-5": "Claude Sonnet 5 — higher quality",
  "gpt-4.1-nano": "gpt-4.1-nano — cheapest",
  "gpt-4o-mini": "gpt-4o-mini",
};
const modelLabel = (m: string) => MODEL_LABEL[m] ?? m;

const ALL_PROVIDERS: Provider[] = ["openai", "anthropic", "groq"];

const PURE_MODIFIERS = [
  "ControlLeft", "ControlRight", "AltLeft", "AltRight",
  "ShiftLeft", "ShiftRight", "MetaLeft", "MetaRight", "OSLeft", "OSRight",
];

const MOD_LABEL: Record<string, string> = {
  control: "Ctrl", ctrl: "Ctrl", alt: "Alt", shift: "Shift",
  super: "Win", meta: "Win", cmd: "Cmd",
};

function prettifyHotkey(accel: string): string {
  if (!accel) return "(none)";
  return accel
    .split("+")
    .map((t) => {
      const low = t.toLowerCase();
      if (MOD_LABEL[low]) return MOD_LABEL[low];
      if (t.startsWith("Key")) return t.slice(3);
      if (t.startsWith("Digit")) return t.slice(5);
      return t;
    })
    .join(" + ");
}

export default function App() {
  const [hasKey, setHasKey] = useState<Record<Provider, boolean>>({
    openai: false,
    anthropic: false,
    groq: false,
  });
  const [keyInput, setKeyInput] = useState<Record<Provider, string>>({
    openai: "",
    anthropic: "",
    groq: "",
  });
  const [sttProvider, setSttProvider] = useState<Provider>("openai");
  const [sttModel, setSttModel] = useState("gpt-4o-mini-transcribe");
  const [refineProvider, setRefineProvider] = useState<Provider>("openai");
  const [refineModel, setRefineModel] = useState("gpt-4.1-nano");
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [active, setActive] = useState(0);
  const [vocabulary, setVocabulary] = useState("");
  const [hotkey, setHotkey] = useState("");
  const [capturing, setCapturing] = useState(false);
  const [status, setStatus] = useState<Status>("idle");
  const [last, setLast] = useState("");
  const [error, setError] = useState("");
  const [saved, setSaved] = useState("");

  useEffect(() => {
    ALL_PROVIDERS.forEach((p) => {
      invoke<boolean>("has_api_key", { provider: p })
        .then((v) => setHasKey((h) => ({ ...h, [p]: v })))
        .catch(() => {});
    });
    invoke<Config>("get_config")
      .then((cfg) => {
        setProfiles(cfg.profiles);
        setActive(cfg.active_profile);
        setVocabulary(cfg.vocabulary);
        setSttProvider(cfg.stt_provider);
        setSttModel(cfg.stt_model);
        setRefineProvider(cfg.refine_provider);
        setRefineModel(cfg.refine_model);
        setHotkey(cfg.hotkey);
      })
      .catch(() => {});
    invoke<string>("get_status").then((s) => setStatus(s as Status)).catch(() => {});

    const subs = [
      listen<string>("status", (e) => setStatus(e.payload as Status)),
      listen<string>("result", (e) => {
        setLast(e.payload);
        setError("");
      }),
      listen<string>("error", (e) => setError(e.payload)),
      listen<string>("info", (e) => flash(e.payload)),
    ];
    return () => {
      subs.forEach((p) => p.then((un) => un()));
    };
  }, []);

  function flash(msg: string) {
    setSaved(msg);
    setTimeout(() => setSaved(""), 2500);
  }

  // --- API keys (per provider) ---
  async function saveKey(p: Provider) {
    const key = keyInput[p];
    try {
      await invoke("set_api_key", { provider: p, key });
      setKeyInput((k) => ({ ...k, [p]: "" }));
      setHasKey((h) => ({ ...h, [p]: true }));
      flash(`${PROVIDER_LABEL[p]} API key saved to the OS keychain.`);
    } catch (e) {
      setError(String(e));
    }
  }

  // --- Voice / STT model ---
  async function persistStt(provider: Provider, model: string) {
    try {
      await invoke("set_stt_config", { provider, model });
      flash("Voice model saved.");
    } catch (e) {
      setError(String(e));
    }
  }
  async function changeSttProvider(provider: Provider) {
    const model = STT_CATALOG.find((c) => c.provider === provider)!.models[0];
    setSttProvider(provider);
    setSttModel(model);
    await persistStt(provider, model);
  }
  async function changeSttModel(model: string) {
    setSttModel(model);
    await persistStt(sttProvider, model);
  }

  // --- Refinement model ---
  async function persistRefine(provider: Provider, model: string) {
    try {
      await invoke("set_refine_config", { provider, model });
      flash("Refinement model saved.");
    } catch (e) {
      setError(String(e));
    }
  }
  async function changeRefineProvider(provider: Provider) {
    const model = REFINE_CATALOG.find((c) => c.provider === provider)!.models[0];
    setRefineProvider(provider);
    setRefineModel(model);
    await persistRefine(provider, model);
  }
  async function changeRefineModel(model: string) {
    setRefineModel(model);
    await persistRefine(refineProvider, model);
  }

  // --- Profiles ---
  async function changeActive(idx: number) {
    setActive(idx);
    try {
      await invoke("set_active_profile", { index: idx });
    } catch (e) {
      setError(String(e));
    }
  }

  function editActive(field: "name" | "prompt", value: string) {
    setProfiles((ps) => ps.map((p, i) => (i === active ? { ...p, [field]: value } : p)));
  }

  async function saveProfiles(next = profiles, nextActive = active) {
    try {
      await invoke("set_profiles", { profiles: next, active: nextActive });
      flash("Profiles saved.");
    } catch (e) {
      setError(String(e));
    }
  }

  async function addProfile() {
    const next = [...profiles, { name: "New profile", prompt: "" }];
    const idx = next.length - 1;
    setProfiles(next);
    setActive(idx);
    await saveProfiles(next, idx);
  }

  async function deleteProfile() {
    if (profiles.length <= 1) {
      setError("Keep at least one profile.");
      return;
    }
    const next = profiles.filter((_, i) => i !== active);
    const idx = Math.min(active, next.length - 1);
    setProfiles(next);
    setActive(idx);
    await saveProfiles(next, idx);
  }

  // --- Vocabulary ---
  async function saveVocabulary() {
    try {
      await invoke("set_vocabulary", { vocabulary });
      flash("Vocabulary saved.");
    } catch (e) {
      setError(String(e));
    }
  }

  // --- Hotkey ---
  function onHotkeyKeyDown(e: React.KeyboardEvent) {
    e.preventDefault();
    if (PURE_MODIFIERS.includes(e.code)) return;
    const mods: string[] = [];
    if (e.ctrlKey) mods.push("control");
    if (e.altKey) mods.push("alt");
    if (e.shiftKey) mods.push("shift");
    if (e.metaKey) mods.push("super");
    if (mods.length === 0) {
      setError("A global hotkey needs at least one modifier (Ctrl/Alt/Shift/Win).");
      return;
    }
    setHotkey([...mods, e.code].join("+"));
    (e.target as HTMLElement).blur();
  }

  async function saveHotkey() {
    try {
      const applied = await invoke<string>("set_hotkey", { accelerator: hotkey });
      setHotkey(applied);
      flash("Hotkey updated.");
    } catch (e) {
      setError(String(e));
    }
  }

  async function toggle() {
    try {
      await invoke("trigger_toggle");
    } catch (e) {
      setError(String(e));
    }
  }

  const activeProfile = profiles[active];
  const sttModels = STT_CATALOG.find((c) => c.provider === sttProvider)?.models ?? [sttModel];
  const refineModels = REFINE_CATALOG.find((c) => c.provider === refineProvider)?.models ?? [refineModel];
  // Only the providers actually selected across the two stages need a key.
  const providersInUse = Array.from(new Set<Provider>([sttProvider, refineProvider]));

  return (
    <main className="container">
      <header>
        <h1>VoiceType AI</h1>
        <span className={`status status-${status}`}>{STATUS_LABEL[status]}</span>
      </header>

      <p className="hint">
        Global hotkey: <kbd>{prettifyHotkey(hotkey)}</kbd> — press once to start, again to stop.
        Refined with the <strong>{activeProfile?.name ?? "active"}</strong> profile.
      </p>

      <section className="card">
        <h2>Models</h2>

        <label className="field-label">Voice (speech-to-text)</label>
        <div className="row">
          <select value={sttProvider} onChange={(e) => changeSttProvider(e.target.value as Provider)}>
            {STT_CATALOG.map((c) => (
              <option key={c.provider} value={c.provider}>
                {c.label}
              </option>
            ))}
          </select>
          <select value={sttModel} onChange={(e) => changeSttModel(e.target.value)}>
            {sttModels.map((m) => (
              <option key={m} value={m}>
                {modelLabel(m)}
              </option>
            ))}
          </select>
        </div>

        <label className="field-label" style={{ marginTop: 12 }}>
          Refinement (AI rewrite)
        </label>
        <div className="row">
          <select
            value={refineProvider}
            onChange={(e) => changeRefineProvider(e.target.value as Provider)}
          >
            {REFINE_CATALOG.map((c) => (
              <option key={c.provider} value={c.provider}>
                {c.label}
              </option>
            ))}
          </select>
          <select value={refineModel} onChange={(e) => changeRefineModel(e.target.value)}>
            {refineModels.map((m) => (
              <option key={m} value={m}>
                {modelLabel(m)}
              </option>
            ))}
          </select>
        </div>

        <p className="muted">
          Anthropic (Claude) can only refine text — it has no voice model, so transcription always
          uses OpenAI or Groq. Groq's Whisper is free (no card required).
        </p>
      </section>

      <section className="card">
        <h2>API keys</h2>
        <p className="muted" style={{ marginTop: 0 }}>
          You only need a key for the provider(s) you selected above. Keys are stored in the OS
          keychain, never in plaintext.
        </p>
        {providersInUse.map((p) => (
          <div key={p} style={{ marginTop: 10 }}>
            <label className="field-label">
              {PROVIDER_LABEL[p]} {hasKey[p] && <span className="ok">✓ set</span>}
            </label>
            <div className="row">
              <input
                type="password"
                placeholder={hasKey[p] ? "•••••••••• (stored)" : KEY_PLACEHOLDER[p]}
                value={keyInput[p]}
                onChange={(e) => setKeyInput((k) => ({ ...k, [p]: e.target.value }))}
              />
              <button onClick={() => saveKey(p)} disabled={!keyInput[p].trim()}>
                Save
              </button>
            </div>
            <p className="muted">Get one at {KEY_HELP[p]}.</p>
          </div>
        ))}
      </section>

      <section className="card">
        <h2>Refinement profiles</h2>
        <div className="row">
          <select value={active} onChange={(e) => changeActive(Number(e.target.value))}>
            {profiles.map((p, i) => (
              <option key={i} value={i}>
                {p.name || `Profile ${i + 1}`}
              </option>
            ))}
          </select>
          <button className="ghost" onClick={addProfile}>
            + Add
          </button>
          <button className="ghost" onClick={deleteProfile} disabled={profiles.length <= 1}>
            Delete
          </button>
        </div>

        {activeProfile && (
          <>
            <input
              style={{ marginTop: 8 }}
              value={activeProfile.name}
              onChange={(e) => editActive("name", e.target.value)}
              placeholder="Profile name"
            />
            <textarea
              style={{ marginTop: 8 }}
              rows={6}
              value={activeProfile.prompt}
              onChange={(e) => editActive("prompt", e.target.value)}
              placeholder="System prompt that shapes how this profile rewrites your dictation…"
            />
            <div className="row">
              <button onClick={() => saveProfiles()}>Save profile</button>
              <button className="ghost" onClick={toggle}>
                Test dictation (toggle)
              </button>
            </div>
          </>
        )}
        <p className="muted">
          The active profile shapes how dictation is rewritten. Switch it anytime — the choice is
          saved automatically.
        </p>
      </section>

      <section className="card">
        <h2>Custom vocabulary</h2>
        <textarea
          rows={3}
          value={vocabulary}
          onChange={(e) => setVocabulary(e.target.value)}
          placeholder="Names, acronyms, product terms — e.g. VoiceType, Devaxl, gpt-4.1-nano, Tauri"
        />
        <div className="row">
          <button onClick={saveVocabulary}>Save vocabulary</button>
        </div>
        <p className="muted">Sent to the transcription model as a hint so it spells your jargon right.</p>
      </section>

      <section className="card">
        <h2>Global hotkey</h2>
        <div className="row">
          <div
            className={`hotkey-capture ${capturing ? "capturing" : ""}`}
            tabIndex={0}
            onKeyDown={onHotkeyKeyDown}
            onFocus={() => setCapturing(true)}
            onBlur={() => setCapturing(false)}
          >
            {capturing ? "Press a key combination…" : prettifyHotkey(hotkey)}
          </div>
          <button onClick={saveHotkey} disabled={!hotkey}>
            Save
          </button>
        </div>
        <p className="muted">Click the box, then press your combination (must include a modifier).</p>
      </section>

      {last && (
        <section className="card">
          <h2>Last refined output</h2>
          <pre className="output">{last}</pre>
        </section>
      )}

      {error && <div className="banner error">{error}</div>}
      {saved && <div className="banner ok-banner">{saved}</div>}
    </main>
  );
}
