import { ChevronsLeft, ChevronLeft, ChevronRight, ChevronsRight } from "lucide-react";
import { cn } from "@/lib/cn";

interface ScrubberProps {
  selectedIndex: number;
  maxIndex: number; // last move index (moves.length - 1)
  onSelect: (i: number) => void;
}

export function TimelineScrubber({ selectedIndex, maxIndex, onSelect }: ScrubberProps) {
  const total = maxIndex + 1; // number of moves
  const pct = total === 0 ? 0 : ((selectedIndex + 1) / total) * 100;

  return (
    <div className="flex items-center gap-3 rounded-md border border-border bg-bg-1 px-3 py-2">
      <div className="flex items-center gap-1">
        <ScrubBtn onClick={() => onSelect(-1)} disabled={selectedIndex === -1} aria-label="start">
          <ChevronsLeft className="size-4" />
        </ScrubBtn>
        <ScrubBtn onClick={() => onSelect(Math.max(-1, selectedIndex - 1))} disabled={selectedIndex === -1}>
          <ChevronLeft className="size-4" />
        </ScrubBtn>
        <ScrubBtn onClick={() => onSelect(Math.min(maxIndex, selectedIndex + 1))} disabled={selectedIndex === maxIndex}>
          <ChevronRight className="size-4" />
        </ScrubBtn>
        <ScrubBtn onClick={() => onSelect(maxIndex)} disabled={selectedIndex === maxIndex}>
          <ChevronsRight className="size-4" />
        </ScrubBtn>
      </div>

      <div className="relative flex-1">
        <input
          type="range"
          min={-1}
          max={maxIndex}
          value={selectedIndex}
          onChange={(e) => onSelect(parseInt(e.target.value, 10))}
          className="w-full accent-[var(--color-accent)]"
        />
        <div className="absolute inset-x-0 -bottom-0.5 h-[2px] rounded bg-bg-3">
          <div className="h-full rounded bg-accent/60" style={{ width: `${pct}%` }} />
        </div>
      </div>

      <div className="min-w-[5rem] text-right font-mono text-xs text-ink-muted">
        {selectedIndex === -1 ? "start" : `${selectedIndex + 1}`} / {total}
      </div>
    </div>
  );
}

function ScrubBtn({
  children,
  disabled,
  onClick,
  ...rest
}: React.ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={cn(
        "grid size-7 place-items-center rounded-md text-ink-muted transition-colors",
        "hover:bg-bg-2 hover:text-ink-strong",
        "disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent",
      )}
      {...rest}
    >
      {children}
    </button>
  );
}
