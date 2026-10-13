import { useState } from "react";

interface Props {
  label: string;
  value: number;
  max: number;
  step: number;
  disabled?: boolean;
  /** Called continuously while the user drags. */
  onLive?: (value: number) => void;
  /** Called once when the user lets go. */
  onCommit?: (value: number) => void;
}

/** A slider that follows the user's finger while dragging, and the real
 *  value the rest of the time. */
export function Slider({
  label,
  value,
  max,
  step,
  disabled,
  onLive,
  onCommit,
}: Props) {
  const [drag, setDrag] = useState<number | null>(null);

  function finish() {
    if (drag !== null) {
      onCommit?.(drag);
      setDrag(null);
    }
  }

  return (
    <input
      type="range"
      aria-label={label}
      min={0}
      max={max}
      step={step}
      disabled={disabled}
      value={drag ?? Math.min(value, max)}
      onChange={(e) => {
        const v = Number(e.target.value);
        setDrag(v);
        onLive?.(v);
      }}
      onPointerUp={finish}
      onKeyUp={finish}
      onBlur={finish}
    />
  );
}
