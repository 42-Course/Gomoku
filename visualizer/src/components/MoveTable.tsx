import { useEffect, useRef } from "react";
import { Bot, User, Scissors } from "lucide-react";
import type { Move } from "@/api/types";
import { coordLabel, formatMs } from "@/lib/format";
import { cn } from "@/lib/cn";

interface MoveTableProps {
  moves: Move[];
  /** Highlighted row. Omit (or -1) for a plain history with no selection. */
  selectedIndex?: number;
  /** When omitted the table is read-only (rows aren't clickable) — used by
   *  the live Play screen, which has no scrubber to jump moves. */
  onSelect?: (i: number) => void;
}

export function MoveTable({ moves, selectedIndex = -1, onSelect }: MoveTableProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const activeRef = useRef<HTMLButtonElement>(null);

  // Interactive (review): keep the selected row in view as the user navigates.
  // Read-only (live play): keep the newest move in view as moves arrive.
  useEffect(() => {
    if (onSelect) {
      activeRef.current?.scrollIntoView({ block: "nearest" });
    } else if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [selectedIndex, moves.length, onSelect]);

  // Pair moves into rounds: [black, white]
  const rounds: [Move, Move | undefined][] = [];
  for (let i = 0; i < moves.length; i += 2) {
    rounds.push([moves[i], moves[i + 1]]);
  }

  return (
    <div className="flex min-h-0 flex-col overflow-hidden rounded-md border border-border bg-bg-1">
      <div className="grid grid-cols-[2.5rem_1fr_1fr] gap-2 border-b border-border bg-bg-2 px-3 py-1.5 text-[10px] font-medium uppercase tracking-wider text-ink-muted">
        <span>#</span>
        <span>Black</span>
        <span>White</span>
      </div>

      <div ref={scrollRef} className="max-h-[480px] overflow-y-auto">
        {rounds.length === 0 && (
          <div className="px-3 py-3 text-xs text-ink-muted">No moves yet.</div>
        )}
        {rounds.map(([b, w], i) => (
          <div
            key={i}
            className="grid grid-cols-[2.5rem_1fr_1fr] gap-2 border-b border-border/50 px-3 py-1.5 text-xs last:border-b-0"
          >
            <span className="font-mono text-ink-muted">{i + 1}</span>
            <MoveCell move={b} isSelected={selectedIndex === b.index} onSelect={onSelect} activeRef={activeRef} />
            {w ? (
              <MoveCell move={w} isSelected={selectedIndex === w.index} onSelect={onSelect} activeRef={activeRef} />
            ) : (
              <span />
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

function MoveCell({
  move,
  isSelected,
  onSelect,
  activeRef,
}: {
  move: Move;
  isSelected: boolean;
  onSelect?: (i: number) => void;
  activeRef: React.RefObject<HTMLButtonElement | null>;
}) {
  const body = (
    <>
      <span className="flex items-center gap-1.5">
        {move.source === "ai" ? (
          <Bot className="size-3 text-accent" />
        ) : (
          <User className="size-3 text-ink-muted" />
        )}
        <span className="font-mono">{coordLabel(move.coord)}</span>
        {move.captured.length > 0 && (
          <Scissors className="size-3 text-accent-2" aria-label="capture" />
        )}
      </span>
      <span className="font-mono text-[10px] text-ink-muted">{formatMs(move.thinkMs)}</span>
    </>
  );

  const base = "flex items-center justify-between gap-2 rounded px-2 py-0.5 text-left";

  // Read-only history (Play): static row, still highlight the latest move.
  if (!onSelect) {
    return (
      <div
        className={cn(
          base,
          isSelected && "bg-accent/15 text-ink-strong ring-1 ring-inset ring-accent/30",
        )}
      >
        {body}
      </div>
    );
  }

  return (
    <button
      ref={isSelected ? activeRef : undefined}
      onClick={() => onSelect(move.index)}
      className={cn(
        base,
        "transition-colors hover:bg-bg-2",
        isSelected && "bg-accent/20 text-ink-strong ring-1 ring-inset ring-accent/40",
      )}
    >
      {body}
    </button>
  );
}
