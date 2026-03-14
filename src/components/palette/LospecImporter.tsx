import { type Component, createSignal, Show } from "solid-js";
import { fetchLospecPalette } from "@/lib/tauri";
import { replaceAllColors } from "@/stores/palette";

const LospecImporter: Component = () => {
  const [slug, setSlug] = createSignal("");
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [importedName, setImportedName] = createSignal<string | null>(null);

  const handleImport = async () => {
    const s = slug().trim();
    if (!s) return;

    setLoading(true);
    setError(null);
    setImportedName(null);

    try {
      const result = await fetchLospecPalette(s);
      replaceAllColors(result.colors, result.name);
      setImportedName(result.name);
      setSlug("");
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
    } finally {
      setLoading(false);
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      handleImport();
    }
  };

  return (
    <div class="lospec-importer">
      <div class="lospec-importer-row">
        <input
          type="text"
          class="lospec-input"
          placeholder="Palette slug (e.g. pear36)"
          value={slug()}
          onInput={(e) => setSlug(e.currentTarget.value)}
          onKeyDown={handleKeyDown}
          disabled={loading()}
        />
        <button
          class="btn-primary lospec-btn"
          onClick={handleImport}
          disabled={loading() || !slug().trim()}
        >
          {loading() ? "..." : "Import"}
        </button>
      </div>
      <div class="lospec-warning">
        This will replace the current palette. Color indices will be preserved.
      </div>
      <Show when={error()}>
        <div class="lospec-error">{error()}</div>
      </Show>
      <Show when={importedName()}>
        <div class="lospec-success">Imported: {importedName()}</div>
      </Show>
    </div>
  );
};

export default LospecImporter;
