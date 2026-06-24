import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

type HudState = "hidden" | "recording" | "processing" | "success" | "error";

const DOT: Record<HudState, string> = {
  hidden: "",
  recording: "●",
  processing: "…",
  success: "✓",
  error: "⚠",
};

export default function Hud() {
  const [state, setState] = useState<HudState>("hidden");
  const [msg, setMsg] = useState("");
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    function show(next: HudState, text: string, autoHideMs?: number) {
      if (timer.current) window.clearTimeout(timer.current);
      setState(next);
      setMsg(text);
      if (autoHideMs) {
        timer.current = window.setTimeout(() => setState("hidden"), autoHideMs);
      }
    }

    const subs = [
      listen<string>("status", (e) => {
        if (e.payload === "recording") show("recording", "Recording");
        else if (e.payload === "processing") show("processing", "Processing");
      }),
      listen<string>("result", () => show("success", "Inserted", 1500)),
      listen<string>("info", (e) => show("success", e.payload, 4500)),
      listen<string>("error", (e) => show("error", e.payload, 5000)),
    ];

    return () => {
      subs.forEach((p) => p.then((un) => un()));
      if (timer.current) window.clearTimeout(timer.current);
    };
  }, []);

  if (state === "hidden") return null;

  return (
    <div className={`hud hud-${state}`}>
      <span className="hud-dot">{DOT[state]}</span>
      <span className="hud-msg">{msg}</span>
    </div>
  );
}
