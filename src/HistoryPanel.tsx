import { useEffect, useRef, useState } from "react";
import { Button } from "./components/ui/button";
import { errorMessage, nativeAvailable } from "./capture-api";
import {
  exportAnki,
  onHistoryChanged,
  queryHistory,
  setStudyFlag,
  type HistoryEntry,
  type HistoryPage,
} from "./history-api";

const historyDateFormat = new Intl.DateTimeFormat("uk-UA", {
  dateStyle: "short",
  timeStyle: "short",
  timeZone: "Europe/Kyiv",
});
const formatHistoryDate = (value: string) => historyDateFormat.format(new Date(value));

export function HistoryPanel() {
  const [data, setData] = useState<HistoryPage | null>(null);
  const [search, setSearch] = useState("");
  const [query, setQuery] = useState("");
  const [page, setPage] = useState(1);
  const [onlyStudy, setOnlyStudy] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [changing, setChanging] = useState<number | null>(null);
  const [exporting, setExporting] = useState(false);
  const [message, setMessage] = useState("");
  const [refresh, setRefresh] = useState(0);
  // A newer search/page request supersedes any previous response.
  const revision = useRef(0);

  useEffect(() => {
    const timer = setTimeout(() => {
      setQuery(search);
      setPage(1);
    }, 250);
    return () => clearTimeout(timer);
  }, [search]);

  useEffect(() => {
    if (!nativeAvailable) return;
    let disposed = false;
    let cleanup: (() => void) | undefined;
    void onHistoryChanged(() => {
      if (!disposed) setRefresh((value) => value + 1);
    }).then((stop) => {
      if (disposed) stop();
      else cleanup = stop;
    });
    return () => {
      disposed = true;
      cleanup?.();
    };
  }, []);

  useEffect(() => {
    if (!nativeAvailable) return;
    let disposed = false;
    const request = ++revision.current;
    setLoading(true);
    setError("");
    void queryHistory(page, query, onlyStudy)
      .then((value) => {
        if (!disposed && revision.current === request) {
          setData(value);
          if (value.page !== page) setPage(value.page);
        }
      })
      .catch((value) => {
        if (!disposed && revision.current === request) setError(errorMessage(value));
      })
      .finally(() => {
        if (!disposed && revision.current === request) setLoading(false);
      });
    return () => {
      disposed = true;
    };
  }, [page, query, onlyStudy, refresh]);

  async function updateStudyFlag(entry: HistoryEntry, selected: boolean) {
    setChanging(entry.id);
    setError("");
    try {
      await setStudyFlag(entry.id, selected);
      setRefresh((value) => value + 1);
    } catch (value) {
      setError(errorMessage(value));
    } finally {
      setChanging(null);
    }
  }

  async function exportStudyPhrases() {
    setExporting(true);
    setError("");
    setMessage("");
    try {
      const result = await exportAnki();
      if (result) setMessage(`Експортовано ${result.count} фраз. Файл: ${result.path}`);
    } catch (value) {
      setError(errorMessage(value));
    } finally {
      setExporting(false);
    }
  }

  const pages = Math.max(1, Math.ceil((data?.total ?? 0) / (data?.pageSize ?? 20)));

  return (
    <section className="history-panel" aria-labelledby="history-heading">
      <h2 id="history-heading">Історія перекладів</h2>
      <p className="muted">Познач корисні фрази для навчання й експортуй їх в Anki.</p>
      <div className="history-tools">
        <label className="search-label" htmlFor="history-search">
          Пошук
          <input
            id="history-search"
            type="search"
            value={search}
            maxLength={500}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Англійський текст або переклад"
          />
        </label>
        <label className="study-filter">
          <input
            type="checkbox"
            checked={onlyStudy}
            onChange={(event) => {
              setOnlyStudy(event.target.checked);
              setPage(1);
            }}
          />
          Лише для навчання
        </label>
      </div>
      <div className="history-export">
        <Button
          onClick={() => void exportStudyPhrases()}
          disabled={exporting || !data?.selectedCount || !nativeAvailable}
        >
          {exporting
            ? "Зберігаємо файл…"
            : `Експорт в Anki${data?.selectedCount ? ` (${data.selectedCount})` : ""}`}
        </Button>
        <p className="muted help">До експорту увійдуть усі позначені записи історії.</p>
      </div>
      {error && (
        <p role="alert" className="error">
          {error}{" "}
          <Button variant="ghost" size="sm" onClick={() => setRefresh((value) => value + 1)}>
            Оновити
          </Button>
        </p>
      )}
      {message && (
        <p role="status" className="export-message">
          {message}
        </p>
      )}
      {loading && (
        <p role="status" className="muted">
          Завантажуємо історію…
        </p>
      )}
      {!loading && !error && !data?.entries.length && (
        <p className="muted empty">
          {query || onlyStudy
            ? "За цими умовами записів немає. Зміни пошук або фільтр."
            : "Історія порожня. Переклади текст, щоб зберегти його тут."}
        </p>
      )}
      <ol className="history-list" aria-busy={loading}>
        {data?.entries.map((entry) => (
          <li key={entry.id}>
            <div className="history-meta">
              <time dateTime={entry.updatedAt}>{formatHistoryDate(entry.updatedAt)}</time>
              <span>{entry.model}</span>
              <label>
                <input
                  type="checkbox"
                  checked={entry.selectedForStudy}
                  disabled={loading || changing !== null}
                  onChange={(event) => void updateStudyFlag(entry, event.target.checked)}
                />
                Для навчання
              </label>
            </div>
            <p className="history-source" lang="en">
              {entry.sourceText}
            </p>
            <p className="history-translation">{entry.translation}</p>
          </li>
        ))}
      </ol>
      {!!data?.total && (
        <div className="history-pagination">
          <Button
            variant="outline"
            size="sm"
            disabled={loading || page <= 1}
            onClick={() => setPage((value) => value - 1)}
          >
            Назад
          </Button>
          <span>
            Сторінка {data.page} / {pages} · Записів: {data.total}
          </span>
          <Button
            variant="outline"
            size="sm"
            disabled={loading || page >= pages}
            onClick={() => setPage((value) => value + 1)}
          >
            Далі
          </Button>
        </div>
      )}
      {data?.storagePath && (
        <details className="storage-details">
          <summary>Файл локальної історії</summary>
          <p className="muted">Для резервної копії закрий RustLens і скопіюй папку цього файлу:</p>
          <code>{data.storagePath}</code>
        </details>
      )}
    </section>
  );
}
