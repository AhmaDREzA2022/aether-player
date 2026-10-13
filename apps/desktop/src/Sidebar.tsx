export type View = "library" | "favorites" | "recent" | "mostPlayed";

const ITEMS: { view: View; label: string }[] = [
  { view: "library", label: "Library" },
  { view: "favorites", label: "Favorites" },
  { view: "recent", label: "Recently played" },
  { view: "mostPlayed", label: "Most played" },
];

interface Props {
  view: View;
  onSelect: (view: View) => void;
}

export function Sidebar({ view, onSelect }: Props) {
  return (
    <nav className="sidebar" aria-label="Views">
      {ITEMS.map((item) => (
        <button
          key={item.view}
          className={item.view === view ? "active" : undefined}
          aria-current={item.view === view ? "page" : undefined}
          onClick={() => onSelect(item.view)}
        >
          {item.label}
        </button>
      ))}
    </nav>
  );
}