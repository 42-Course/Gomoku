import { Crosshair, Hash, Mouse } from "lucide-react";
import { cn } from "@/lib/cn";

interface BoardSettingsProps {
  showCoordinates: boolean;
  showCrosshair: boolean;
  showHoverGhost: boolean;
  onToggleCoordinates: () => void;
  onToggleCrosshair: () => void;
  onToggleHoverGhost: () => void;
}

/**
 * Compact toggle row for the board's display chrome.
 *
 * Lives next to the board on every page that renders it. The toggles
 * themselves are persisted in the gameView store so a preference set on
 * /play follows the user to /games/:id and back.
 */
export function BoardSettings({
  showCoordinates,
  showCrosshair,
  showHoverGhost,
  onToggleCoordinates,
  onToggleCrosshair,
  onToggleHoverGhost,
}: BoardSettingsProps) {
  return (
    <div className="flex items-center gap-1.5 rounded-md border border-border bg-bg-1 px-2 py-1.5">
      <span className="mr-1 text-[10px] uppercase tracking-wider text-ink-muted">
        Board
      </span>
      <Pill active={showCoordinates} onClick={onToggleCoordinates} icon={<Hash className="size-3" />}>
        Coords
      </Pill>
      <Pill active={showCrosshair} onClick={onToggleCrosshair} icon={<Crosshair className="size-3" />}>
        Crosshair
      </Pill>
      <Pill active={showHoverGhost} onClick={onToggleHoverGhost} icon={<Mouse className="size-3" />}>
        Ghost
      </Pill>
    </div>
  );
}

function Pill({
  active,
  icon,
  children,
  ...rest
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  active: boolean;
  icon: React.ReactNode;
}) {
  return (
    <button
      {...rest}
      className={cn(
        "inline-flex items-center gap-1 rounded px-2 py-0.5 text-[11px] transition-colors",
        active
          ? "bg-accent/15 text-accent"
          : "text-ink-muted hover:bg-bg-2 hover:text-ink-strong",
      )}
    >
      {icon}
      {children}
    </button>
  );
}
