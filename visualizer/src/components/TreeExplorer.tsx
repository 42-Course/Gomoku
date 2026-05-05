import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight, GitBranch } from "lucide-react";
import type { Coord, TreeNode } from "@/api/types";
import { coordLabel } from "@/lib/format";
import { principalVariation } from "@/engine/adapters";
import { cn } from "@/lib/cn";

interface TreeExplorerProps {
  root: TreeNode | null | undefined;
  isLoading?: boolean;
  onHoverCoord: (c: Coord | null) => void;
}

/**
 * Renders the engine's flattened search tree as a collapsible outline.
 *
 * Each row exposes the four facts a min-max + α-β reader cares about:
 *
 *   • the move that led to the node (or "root" at the top)
 *   • the score the node returned (in the root mover's frame)
 *   • the [α, β] window the node was called with
 *   • whether the node terminated on a β-cutoff (`pruned`)
 *
 * Nodes on the principal variation are marked with a chevron and the
 * accent color, so the chain of "best play assuming both sides play their
 * best" reads at a glance.
 */
export function TreeExplorer({
  root,
  isLoading,
  onHoverCoord,
}: TreeExplorerProps) {
  const pvSet = useMemo(() => {
    const pv = principalVariation(root ?? null);
    return new Set(pv.map((c) => `${c.x},${c.y}`));
  }, [root]);

  return (
    <div className="overflow-hidden rounded-md border border-border bg-bg-1">
      <div className="flex items-center justify-between border-b border-border bg-bg-2 px-3 py-1.5">
        <div className="flex items-center gap-2 text-xs font-medium text-ink-strong">
          <GitBranch className="size-3.5 text-accent" /> Min-max tree
        </div>
        <span className="font-mono text-[10px] text-ink-muted">
          α-β · score · [α, β]
        </span>
      </div>

      <div className="max-h-[420px] overflow-auto p-2 font-mono text-xs">
        {isLoading && <div className="p-3 text-ink-muted">Building tree…</div>}
        {!isLoading && !root && (
          <div className="p-3 text-ink-muted">
            No tree available for this move.
          </div>
        )}
        {root && (
          <TreeNodeView
            node={root}
            depth={0}
            onPv={pvSet}
            onHoverCoord={onHoverCoord}
            isRoot
          />
        )}
      </div>
    </div>
  );
}

function TreeNodeView({
  node,
  depth,
  onPv,
  onHoverCoord,
  isRoot = false,
}: {
  node: TreeNode;
  depth: number;
  onPv: Set<string>;
  onHoverCoord: (c: Coord | null) => void;
  isRoot?: boolean;
}) {
  const [open, setOpen] = useState(depth < 1);
  const hasChildren = node.children.length > 0;
  const onPath = !isRoot && onPv.has(`${node.coord.x},${node.coord.y}`);

  return (
    <div>
      <button
        onClick={() => hasChildren && setOpen((o) => !o)}
        onMouseEnter={() => !isRoot && onHoverCoord(node.coord)}
        onMouseLeave={() => onHoverCoord(null)}
        className={cn(
          "flex w-full items-center gap-2 rounded px-1 py-0.5 text-left transition-colors",
          "hover:bg-bg-2",
          node.pruned && "opacity-50",
          onPath && "bg-accent/10",
        )}
        style={{ paddingLeft: `${depth * 14 + 4}px` }}
      >
        <span className="w-3 text-ink-muted">
          {hasChildren ? (
            open ? (
              <ChevronDown className="size-3" />
            ) : (
              <ChevronRight className="size-3" />
            )
          ) : (
            "·"
          )}
        </span>

        <span
          className={cn(
            "inline-block size-2 rounded-full",
            node.player === "black" ? "bg-stone-black" : "bg-stone-white",
          )}
        />

        <span className={cn("text-ink-strong", onPath && "text-accent")}>
          {isRoot ? "root" : coordLabel(node.coord)}
        </span>

        <span className="ml-1 text-[10px] text-ink-muted">
          d{node.depth}
        </span>

        <span className={cn("ml-auto font-mono text-[10px]", scoreColor(node.score))}>
          {formatScore(node.score)}
        </span>

        <span className="font-mono text-[10px] text-ink-muted">
          [{formatBound(node.alpha)}, {formatBound(node.beta)}]
        </span>

        {node.pruned && (
          <span className="rounded bg-danger/20 px-1 py-0 text-[9px] uppercase text-danger">
            β-cut
          </span>
        )}
      </button>

      {open && hasChildren && (
        <div>
          {node.children.map((c) => (
            <TreeNodeView
              key={c.id}
              node={c}
              depth={depth + 1}
              onPv={onPv}
              onHoverCoord={onHoverCoord}
            />
          ))}
        </div>
      )}
    </div>
  );
}

/** Engine sends i32::MIN+1 / i32::MAX-1 as ±∞; render them compactly. */
function formatBound(v: number): string {
  if (v <= -2_000_000_000) return "-∞";
  if (v >= 2_000_000_000) return "+∞";
  return v.toString();
}

function formatScore(score: number): string {
  if (score >= 1_000_000) return "+mate";
  if (score <= -1_000_000) return "-mate";
  return `${score > 0 ? "+" : ""}${score.toFixed(0)}`;
}

function scoreColor(score: number) {
  if (score > 300) return "text-accent";
  if (score < -300) return "text-danger";
  return "text-ink-muted";
}
