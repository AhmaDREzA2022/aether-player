export type ShortcutAction =
  | "toggle"
  | "next"
  | "prev"
  | "seekBack"
  | "seekForward"
  | "mute";

type KeyInfo = Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey" | "repeat">;

/** What a key press means, or null if it isn't ours to handle. */
export function shortcutFor(e: KeyInfo, target: EventTarget | null): ShortcutAction | null {
  if (e.ctrlKey || e.metaKey || e.altKey) return null; // leave Ctrl+R and friends alone

  const el = target instanceof HTMLElement ? target : null;
  const tag = el?.tagName;
  const isRange = el instanceof HTMLInputElement && el.type === "range";

  // Typing in a text field is never a shortcut. Sliders are the exception
  // for letters and Space, but they handle the arrow keys themselves.
  const typing =
    (tag === "INPUT" && !isRange) || tag === "TEXTAREA" || tag === "SELECT" || !!el?.isContentEditable;
  if (typing) return null;

  switch (e.key) {
    case " ":
      // A focused button already turns Space into a click.
      return e.repeat || tag === "BUTTON" ? null : "toggle";
    case "ArrowLeft":
      return isRange ? null : "seekBack";
    case "ArrowRight":
      return isRange ? null : "seekForward";
    case "n":
    case "N":
      return e.repeat ? null : "next";
    case "p":
    case "P":
      return e.repeat ? null : "prev";
    case "m":
    case "M":
      return e.repeat ? null : "mute";
    default:
      return null;
  }
}
