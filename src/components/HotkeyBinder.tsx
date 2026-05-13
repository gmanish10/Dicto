import { useEffect, useState } from "react";

interface Props {
  value: string;
  onChange: (chord: string) => void;
}

const PRESETS: { label: string; chord: string; note?: string }[] = [
  { label: "Fn key", chord: "Fn", note: "macOS: Settings → Keyboard → \"Press 🌐 key to\" must be \"Do Nothing\"" },
  { label: "Right Option", chord: "RightOption", note: "Some keyboards deliver as plain Option — try \"Option\" if this fails" },
  { label: "Left Option", chord: "LeftOption" },
  { label: "Option (either side)", chord: "Option" },
  { label: "Cmd + Shift + Space", chord: "Cmd+Shift+Space" },
  { label: "Cmd + Shift + D", chord: "Cmd+Shift+D" },
  { label: "Cmd + Option + Space", chord: "Cmd+Option+Space" },
  { label: "Control + Space", chord: "Control+Space" },
  { label: "Custom…", chord: "__custom__" },
];

function presetForChord(chord: string): string {
  return PRESETS.find((p) => p.chord === chord)?.chord ?? "__custom__";
}

export function HotkeyBinder({ value, onChange }: Props) {
  const [selected, setSelected] = useState(presetForChord(value));
  const [custom, setCustom] = useState(presetForChord(value) === "__custom__" ? value : "");

  // Keep local state in sync if `value` changes from outside.
  useEffect(() => {
    setSelected(presetForChord(value));
    if (presetForChord(value) === "__custom__") setCustom(value);
  }, [value]);

  const activeNote = PRESETS.find((p) => p.chord === selected)?.note;

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-3">
        <select
          className="input max-w-xs"
          value={selected}
          onChange={(e) => {
            const next = e.target.value;
            setSelected(next);
            if (next !== "__custom__") {
              onChange(next);
            } else if (custom.trim()) {
              onChange(custom.trim());
            }
          }}
        >
          {PRESETS.map((p) => (
            <option key={p.chord} value={p.chord}>
              {p.label}
            </option>
          ))}
        </select>
        <code className="rounded bg-ink-100 px-3 py-2 font-mono text-sm dark:bg-ink-700">
          {value || "(none)"}
        </code>
      </div>

      {selected === "__custom__" && (
        <div className="flex items-center gap-2">
          <input
            className="input max-w-xs font-mono"
            placeholder="e.g. Cmd+Shift+H"
            value={custom}
            onChange={(e) => setCustom(e.target.value)}
            onBlur={() => {
              if (custom.trim()) onChange(custom.trim());
            }}
          />
          <button
            type="button"
            className="btn-primary text-xs"
            disabled={!custom.trim()}
            onClick={() => onChange(custom.trim())}
          >
            Save
          </button>
        </div>
      )}

      {activeNote && <p className="text-xs text-ink-400">{activeNote}</p>}

      <details className="text-xs text-ink-400">
        <summary className="cursor-pointer">Custom chord syntax</summary>
        <p className="mt-1">
          Use <code className="rounded bg-ink-100 px-1 dark:bg-ink-700">+</code> to combine. Recognized
          parts: <code>Cmd</code>, <code>Shift</code>, <code>Control</code>, <code>Option</code>,
          {" "}<code>LeftOption</code>, <code>RightOption</code>, <code>Fn</code>, plus a single
          letter or <code>Space</code> / <code>Tab</code> / <code>Return</code> / <code>Escape</code>.
        </p>
      </details>
    </div>
  );
}
