import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { ArrowRight, Circle, Clock, CircleDot } from "lucide-react";
import { listGames, queryKeys } from "@/api/client";
import type { GameStatus, GameSummary } from "@/api/types";
import { coordLabel, relativeFromNow } from "@/lib/format";
import { cn } from "@/lib/cn";

function StatusBadge({ status }: { status: GameStatus }) {
  if (status.kind === "ongoing") {
    return (
      <span className="inline-flex items-center gap-1.5 rounded-full bg-accent/15 px-2 py-0.5 text-xs text-accent">
        <span className="size-1.5 rounded-full bg-accent" /> live
      </span>
    );
  }
  if (status.kind === "draw") {
    return (
      <span className="inline-flex items-center gap-1.5 rounded-full bg-bg-3 px-2 py-0.5 text-xs text-ink-muted">
        draw
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-1.5 rounded-full bg-bg-3 px-2 py-0.5 text-xs text-ink-strong">
      {status.player === "black" ? <Circle className="size-3 fill-stone-black" /> : <CircleDot className="size-3" />}
      {status.player} wins
    </span>
  );
}

function KindBadge({ kind }: { kind: GameSummary["kind"] }) {
  if (kind === "fixture") {
    return (
      <span className="inline-flex items-center rounded bg-bg-3 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-ink-muted">
        fixture
      </span>
    );
  }
  return (
    <span className="inline-flex items-center rounded bg-accent/15 px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide text-accent">
      local
    </span>
  );
}

function GameCard({ g }: { g: GameSummary }) {
  return (
    <Link
      to={`/games/${g.id}`}
      className={cn(
        "group flex flex-col gap-3 rounded-lg border border-border bg-bg-1 p-4",
        "transition-all hover:border-border-strong hover:bg-bg-2",
      )}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-medium text-ink-strong">{g.title}</h3>
          <KindBadge kind={g.kind} />
        </div>
        <StatusBadge status={g.status} />
      </div>

      <div className="flex flex-col gap-1 text-xs">
        <div className="flex items-center gap-2">
          <Circle className="size-3 fill-stone-black stroke-0" />
          <span className="text-ink-strong">{g.black}</span>
        </div>
        <div className="flex items-center gap-2">
          <Circle className="size-3 fill-stone-white stroke-0" />
          <span className="text-ink-strong">{g.white}</span>
        </div>
      </div>

      <div className="mt-auto flex items-center justify-between text-xs text-ink-muted">
        <span>
          {g.moveCount} moves
          {g.lastCoord ? ` · last ${coordLabel(g.lastCoord)}` : ""}
        </span>
        <span className="flex items-center gap-1">
          <Clock className="size-3" />
          {relativeFromNow(g.updatedAt, Date.parse("2026-04-22T14:14:00Z"))}
        </span>
      </div>

      <div className="flex items-center gap-1 text-xs text-accent opacity-0 transition-opacity group-hover:opacity-100">
        open <ArrowRight className="size-3" />
      </div>
    </Link>
  );
}

export function GamesList() {
  const { data, isLoading, isError } = useQuery({
    queryKey: queryKeys.games,
    queryFn: listGames,
  });

  return (
    <div className="mx-auto max-w-6xl px-8 py-10">
      <div className="mb-8 flex items-end justify-between">
        <div>
          <h1 className="text-2xl font-semibold text-ink-strong">Games</h1>
          <p className="mt-1 text-sm text-ink-muted">
            Open a game to see the board, move list, and AI analysis.
          </p>
        </div>
        <div className="rounded-md bg-bg-2 px-3 py-1.5 font-mono text-xs text-ink-muted">
          {data?.length ?? "—"} games
        </div>
      </div>

      {isLoading && <div className="text-sm text-ink-muted">Loading…</div>}
      {isError && <div className="text-sm text-danger">Failed to load games.</div>}

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {data?.map((g) => <GameCard key={g.id} g={g} />)}
      </div>
    </div>
  );
}
