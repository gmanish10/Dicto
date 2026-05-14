import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

/**
 * Recording overlay pill rendered inside the dedicated
 * `recording-overlay` Tauri window.
 *
 * The window itself is always-on-top, click-through, and **always kept
 * visible at the OS level** — we never call `window.hide()`. On macOS,
 * a hidden window when a fullscreen Space is activated never gets
 * registered as an auxiliary in that Space; if we later `show()` it,
 * it lands in the regular Space and is invisible behind fullscreen
 * apps. Keeping the window visible-but-empty avoids that.
 *
 * "Show/hide" is therefore a React-level concept: we listen for the
 * `overlay:set-visible` event from Rust and toggle whether the pill
 * paints. When hidden, the component renders nothing — the window is
 * still there but fully transparent, taking no clicks (it's
 * click-through anyway).
 */
export default function RecordingOverlay() {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    // Override the global Tailwind base layer that paints `html, body,
    // #root` with `bg-ink-50`. Without resetting #root the overlay
    // window shows as a solid grey/white rectangle even though the
    // Tauri window itself is transparent — the pill renders on top of
    // an opaque DIV that fills the whole window.
    const root = document.getElementById("root");
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    document.body.style.color = "white";
    if (root) {
      root.style.background = "transparent";
    }

    const unlisten = listen<boolean>("overlay:set-visible", (e) => {
      setVisible(Boolean(e.payload));
    });

    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  if (!visible) return null;

  return (
    <div
      className="flex h-screen w-screen items-center justify-center"
      style={{ background: "transparent" }}
    >
      <div className="flex items-center gap-2 rounded-full bg-black/80 px-3 py-1.5 text-xs font-medium text-white shadow-lg backdrop-blur-md">
        <RecordingDot />
        <span className="tracking-wide">Listening</span>
      </div>
    </div>
  );
}

function RecordingDot() {
  // Recolored to pastel blush in v0.3.0 to match the rest of the
  // palette. The dot still reads as obviously "recording" against the
  // dark overlay pill because blush is high-contrast on near-black.
  return (
    <span className="relative flex h-2.5 w-2.5">
      <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-brand-blush opacity-75" />
      <span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-brand-blush" />
    </span>
  );
}
