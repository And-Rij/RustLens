import { MainView } from "./views/MainView";
import { SelectionView } from "./views/SelectionView";
import { ResultView } from "./views/ResultView";
import "./App.css";

/** Every native window loads this entry point with its own view parameter. */
export default function App() {
  const view = new URLSearchParams(window.location.search).get("view");
  if (view === "selection") return <SelectionView />;
  if (view === "result") return <ResultView />;
  return <MainView />;
}
