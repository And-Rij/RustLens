import { useEffect, useState } from "react";
import { Button } from "./components/ui/button";
import { errorMessage, nativeAvailable } from "./capture-api";
import { getModels, selectModel } from "./translation-api";

export function ModelSettings() {
  const [models, setModels] = useState<string[]>([]);
  const [selected, setSelected] = useState("translategemma:12b");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function refreshModels() {
    setError("");
    setLoading(true);
    try {
      const response = await getModels();
      setModels(response.models);
      setSelected(response.selected);
    } catch (value) {
      setError(errorMessage(value));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    if (nativeAvailable) void refreshModels();
  }, []);

  async function selectTranslationModel(model: string) {
    setError("");
    setLoading(true);
    try {
      await selectModel(model);
      setSelected(model);
    } catch (value) {
      setError(errorMessage(value));
    } finally {
      setLoading(false);
    }
  }

  return (
    <section className="model-settings" aria-label="Модель перекладу">
      <label htmlFor="translation-model">Локальна модель</label>
      <div className="model-controls">
        <select
          id="translation-model"
          value={selected}
          onChange={(event) => void selectTranslationModel(event.target.value)}
          disabled={loading || !models.length}
        >
          {!models.includes(selected) && (
            <option value={selected}>
              {selected}
              {models.length ? " — не встановлена" : ""}
            </option>
          )}
          {models.map((model) => (
            <option key={model} value={model}>
              {model}
            </option>
          ))}
        </select>
        <Button
          variant="outline"
          size="sm"
          disabled={loading || !nativeAvailable}
          onClick={() => void refreshModels()}
        >
          {loading ? "Перевіряємо…" : "Оновити список"}
        </Button>
      </div>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      <p className="muted help">
        Ollama має бути запущена. Вибір моделі діє до перезапуску RustLens.
      </p>
    </section>
  );
}
