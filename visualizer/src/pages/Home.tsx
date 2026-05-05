import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { ArrowRight, Bot, Circle, Clock, Users } from "lucide-react";
import { listGames, queryKeys } from "@/api/client";
import type { GameSummary } from "@/api/types";
import { coordLabel, relativeFromNow } from "@/lib/format";
import { cn } from "@/lib/cn";

/**
 * Lobby. Two big "start a game" CTAs and a strip of recent activity so
 * users can jump back into something they were reviewing.
 */
export function Home() {
  const { data } = useQuery({
    queryKey: queryKeys.games,
    queryFn: listGames,
  });

  const recents = (data ?? []).slice(0, 4);

  return (
    <div className="mx-auto max-w-6xl px-8 py-10">
      <div className="mb-8">
        <h1 className="text-3xl font-semibold text-ink-strong">Welcome.</h1>
        <p className="mt-2 max-w-prose text-sm text-ink-muted">
          Local-first Gomoku with a real engine. Play, review, and watch the
          alpha-beta search in action.
        </p>
      </div>

      <div className="grid gap-4 sm:grid-cols-2">
        <ModeCard
          to="/play?mode=vsai"
          title="Play vs AI"
          subtitle="Choose a depth, see what the engine considered."
          icon={<Bot className="size-5" />}
        />
        <ModeCard
          to="/play?mode=hotseat"
          title="Play vs Human"
          subtitle="Two players, one device. Pass and play."
          icon={<Users className="size-5" />}
        />
      </div>

      <section className="mt-10">
        <div className="mb-3 flex items-end justify-between">
          <h2 className="text-sm font-medium uppercase tracking-wider text-ink-muted">
            Recent
          </h2>
          <Link
            to="/games"
            className="flex items-center gap-1 text-xs text-accent hover:underline"
          >
            All games <ArrowRight className="size-3" />
          </Link>
        </div>
        {recents.length === 0 ? (
          <div className="rounded-md border border-dashed border-border bg-bg-1 p-6 text-center text-sm text-ink-muted">
            No games yet. Start one above.
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {recents.map((g) => (
              <RecentCard key={g.id} g={g} />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

function ModeCard({
  to,
  title,
  subtitle,
  icon,
}: {
  to: string;
  title: string;
  subtitle: string;
  icon: React.ReactNode;
}) {
  return (
    <Link
      to={to}
      className={cn(
        "group flex items-center gap-4 rounded-lg border border-border bg-bg-1 p-5",
        "transition-all hover:border-accent/60 hover:bg-bg-2",
      )}
    >
      <div className="grid size-12 place-items-center rounded-md bg-accent/15 text-accent">
        {icon}
      </div>
      <div className="flex-1">
        <div className="text-base font-medium text-ink-strong">{title}</div>
        <div className="text-xs text-ink-muted">{subtitle}</div>
      </div>
      <ArrowRight className="size-4 text-ink-muted transition-transform group-hover:translate-x-1 group-hover:text-accent" />
    </Link>
  );
}

function RecentCard({ g }: { g: GameSummary }) {
  return (
    <Link
      to={`/games/${g.id}`}
      className={cn(
        "flex flex-col gap-2 rounded-md border border-border bg-bg-1 p-3",
        "transition-colors hover:border-border-strong hover:bg-bg-2",
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <span className="truncate text-sm text-ink-strong">{g.title}</span>
        <span
          className={cn(
            "rounded px-1.5 py-0.5 font-mono text-[10px] uppercase",
            g.kind === "fixture"
              ? "bg-bg-3 text-ink-muted"
              : "bg-accent/15 text-accent",
          )}
        >
          {g.kind}
        </span>
      </div>
      <div className="flex items-center gap-2 text-[11px] text-ink-muted">
        <Circle className="size-2.5 fill-stone-black stroke-0" />
        <span>{g.black}</span>
        <span>vs</span>
        <Circle className="size-2.5 fill-stone-white stroke-0" />
        <span>{g.white}</span>
      </div>
      <div className="flex items-center justify-between text-[11px] text-ink-muted">
        <span>
          {g.moveCount} moves
          {g.lastCoord ? ` · ${coordLabel(g.lastCoord)}` : ""}
        </span>
        <span className="flex items-center gap-1">
          <Clock className="size-3" />
          {relativeFromNow(g.updatedAt)}
        </span>
      </div>
    </Link>
  );
}
