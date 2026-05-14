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
    const prevBg = document.body.style.background;
    const prevColor = document.body.style.color;
    document.body.style.background = "transparent";
    document.body.style.color = "white";
    document.documentElement.style.background = "transparent";

    const unlisten = listen<boolean>("overlay:set-visible", (e) => {
      setVisible(Boolean(e.payload));
    });

    return () => {
      document.body.style.background = prevBg;
      document.body.style.color = prevColor;
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
  return (
    <span className="relative flex h-2.5 w-2.5">
      <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-red-500 opacity-75" />
      <span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-red-500" />
    </span>
  );
}
