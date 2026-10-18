import type { Selection } from "../capture-api";

export function CaptureFragment({ selection }: { selection: Selection }) {
  return (
    <figure className="fragment">
      <img src={selection.imageUrl} alt="Вибрана область екрана" />
      <figcaption>
        {selection.width} × {selection.height} px
      </figcaption>
    </figure>
  );
}
