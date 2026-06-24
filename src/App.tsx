import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Status = "idle" | "recording" | "processing";

interface Profile {
  name: string;
  prompt: string;
}

interface Config {
  profiles: Profile[];
  active_profile: number;
  vocabulary: string;
  hotkey: string;
}

const STATUS_LABEL: Record<Status, string> = {
  idle: "Idle",
  recording: "● Recording…",
  processing: "… Processing",
};

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
  const [hasKey, setHasKey] = useState(false);
  const [apiKey, setApiKey] = useState("");
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
    invoke<boolean>("has_api_key").then(setHasKey).catch(() => {});
    invoke<Config>("get_config")
      .then((cfg) => {
        setProfiles(cfg.profiles);
        setActive(cfg.active_profile);
        setVocabulary(cfg.vocabulary);
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

  async function saveKey() {
    try {
      await invoke("set_api_key", { key: apiKey });
      setApiKey("");
      setHasKey(true);
      flash("API key saved to the OS keychain.");
    } catch (e) {
      setError(String(e));
    }
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
        <h2>OpenAI API key {hasKey && <span className="ok">✓ set</span>}</h2>
        <div className="row">
          <input
            type="password"
            placeholder={hasKey ? "•••••••••• (stored)" : "sk-…"}
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
          />
          <button onClick={saveKey} disabled={!apiKey.trim()}>
            Save
          </button>
        </div>
        <p className="muted">Stored in the OS keychain, never in plaintext.</p>
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
