import { useCallback, useEffect, useState } from "react";
import { CustomWord, Replacement, api } from "../lib/ipc";

export default function Dictionary() {
  const [words, setWords] = useState<CustomWord[]>([]);
  const [replacements, setReplacements] = useState<Replacement[]>([]);
  const [newWord, setNewWord] = useState("");
  const [newWeight, setNewWeight] = useState(5);
  const [newTrigger, setNewTrigger] = useState("");
  const [newReplacement, setNewReplacement] = useState("");
  const [caseSensitive, setCaseSensitive] = useState(false);

  const reload = useCallback(async () => {
    setWords(await api.listDictionaryWords());
    setReplacements(await api.listReplacements());
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  async function addWord() {
    if (!newWord.trim()) return;
    await api.addDictionaryWord(newWord.trim(), newWeight);
    setNewWord("");
    await reload();
  }

  async function addReplacement() {
    if (!newTrigger.trim() || !newReplacement) return;
    await api.upsertReplacement(newTrigger.trim(), newReplacement, caseSensitive);
    setNewTrigger("");
    setNewReplacement("");
    await reload();
  }

  return (
    <div className="space-y-8">
      <section>
        <h2 className="mb-2 text-lg font-semibold">Custom vocabulary</h2>
        <p className="mb-4 text-sm text-ink-500 dark:text-ink-300">
          Words Whisper should know about — product names, jargon, names. These bias the
          transcription model, with a budget of ~150 words across all entries (higher-weight
          entries are kept when truncating).
        </p>

        <div className="card mb-4 flex flex-wrap gap-2">
          <input
            className="input flex-1"
            placeholder="e.g. Kubernetes"
            value={newWord}
            onChange={(e) => setNewWord(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addWord()}
          />
          <input
            className="input w-24"
            type="number"
            min={1}
            max={100}
            value={newWeight}
            onChange={(e) => setNewWeight(parseInt(e.target.value || "1", 10))}
            title="Weight (higher = more prominent in the prompt budget)"
          />
          <button type="button" className="btn-primary" onClick={addWord}>
            Add
          </button>
        </div>

        <div className="space-y-1">
          {words.length === 0 && (
            <p className="text-sm text-ink-400">No custom words yet.</p>
          )}
          {words.map((w) => (
            <div key={w.id} className="flex items-center justify-between rounded-md border border-ink-200 px-3 py-2 dark:border-ink-700">
              <div className="font-mono text-sm">{w.word}</div>
              <div className="flex items-center gap-3 text-xs text-ink-400">
                <span>weight: {w.weight}</span>
                <button
                  type="button"
                  className="text-red-500 hover:underline"
                  onClick={async () => {
                    await api.deleteDictionaryWord(w.id);
                    void reload();
                  }}
                >
                  Delete
                </button>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section>
        <h2 className="mb-2 text-lg font-semibold">Replacements</h2>
        <p className="mb-4 text-sm text-ink-500 dark:text-ink-300">
          Substitute one phrase for another after transcription. Useful for things like
          <code className="mx-1 rounded bg-ink-100 px-1 dark:bg-ink-700">newline</code>
          →
          <code className="mx-1 rounded bg-ink-100 px-1 dark:bg-ink-700">\n</code> or
          <code className="mx-1 rounded bg-ink-100 px-1 dark:bg-ink-700">k8s</code>
          →
          <code className="mx-1 rounded bg-ink-100 px-1 dark:bg-ink-700">Kubernetes</code>.
        </p>
        <div className="card mb-4 flex flex-wrap items-end gap-2">
          <div className="flex-1">
            <label className="label">When I say…</label>
            <input
              className="input"
              value={newTrigger}
              onChange={(e) => setNewTrigger(e.target.value)}
            />
          </div>
          <div className="flex-1">
            <label className="label">Replace with</label>
            <input
              className="input"
              value={newReplacement}
              onChange={(e) => setNewReplacement(e.target.value)}
            />
          </div>
          <label className="flex items-center gap-2 text-xs">
            <input
              type="checkbox"
              checked={caseSensitive}
              onChange={(e) => setCaseSensitive(e.target.checked)}
            />
            Case sensitive
          </label>
          <button type="button" className="btn-primary" onClick={addReplacement}>
            Add
          </button>
        </div>
        <div className="space-y-1">
          {replacements.length === 0 && (
            <p className="text-sm text-ink-400">No replacements yet.</p>
          )}
          {replacements.map((r) => (
            <div key={r.id} className="flex items-center justify-between rounded-md border border-ink-200 px-3 py-2 dark:border-ink-700">
              <div className="text-sm">
                <code className="font-mono">{r.trigger}</code>
                <span className="mx-2 text-ink-400">→</span>
                <code className="font-mono">{r.replacement}</code>
                {r.case_sensitive && <span className="ml-2 pill-yellow">case-sensitive</span>}
              </div>
              <button
                type="button"
                className="text-xs text-red-500 hover:underline"
                onClick={async () => {
                  await api.deleteReplacement(r.id);
                  void reload();
                }}
              >
                Delete
              </button>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}
