import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowRight, Bot, Circle, Clock, Users, X } from "lucide-react";
import { listGames, queryKeys } from "@/api/client";
import type { GameSummary, Player } from "@/api/types";
import { createLocalGame } from "@/storage/games";
import { coordLabel, relativeFromNow } from "@/lib/format";
import { cn } from "@/lib/cn";

/**
 * Lobby. Two big "start a game" CTAs and a strip of recent activity so
 * users can jump back into something they were reviewing — including
 * unfinished games, which are saved the moment they're started.
 */
export function Home() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [aiModalOpen, setAiModalOpen] = useState(false);

  const { data } = useQuery({
    queryKey: queryKeys.games,
    queryFn: listGames,
  });

  const recents = (data ?? []).slice(0, 4);

  const startHotseat = async () => {
    const id = await createLocalGame({ mode: "hotseat" });
    queryClient.invalidateQueries({ queryKey: queryKeys.games });
    navigate(`/play?id=${id}`);
  };

  const startVsAi = async (side: Player, depth: number) => {
    const id = await createLocalGame({
      mode: "vsai",
      aiSide: side === "black" ? "white" : "black",
      aiDepth: depth,
    });
    setAiModalOpen(false);
    queryClient.invalidateQueries({ queryKey: queryKeys.games });
    navigate(`/play?id=${id}`);
  };

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
        <ModeButton
          title="Play vs AI"
          subtitle="Pick a side and depth, then face the engine."
          icon={<Bot className="size-5" />}
          onClick={() => setAiModalOpen(true)}
        />
        <ModeButton
          title="Play vs Human"
          subtitle="Two players, one device. Pass and play."
          icon={<Users className="size-5" />}
          onClick={startHotseat}
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

      {aiModalOpen && (
        <AiSetupModal
          onCancel={() => setAiModalOpen(false)}
          onStart={startVsAi}
        />
      )}
    </div>
  );
}

function ModeButton({
  title,
  subtitle,
  icon,
  onClick,
}: {
  title: string;
  subtitle: string;
  icon: React.ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "group flex items-center gap-4 rounded-lg border border-border bg-bg-1 p-5 text-left",
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
    </button>
  );
}

function RecentCard({ g }: { g: GameSummary }) {
  const isOngoing = g.status.kind === "ongoing";
  const href = isOngoing && g.kind === "local" ? `/play?id=${g.id}` : `/games/${g.id}`;
  return (
    <Link
      to={href}
      className={cn(
        "flex flex-col gap-2 rounded-md border border-border bg-bg-1 p-3",
        "transition-colors hover:border-border-strong hover:bg-bg-2",
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <span className="truncate text-sm text-ink-strong">{g.title}</span>
        <StatusBadge g={g} />
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

function StatusBadge({ g }: { g: GameSummary }) {
  if (g.status.kind === "ongoing" && g.kind === "local") {
    return (
      <span className="rounded bg-accent/20 px-1.5 py-0.5 font-mono text-[10px] uppercase text-accent">
        ongoing
      </span>
    );
  }
  if (g.status.kind === "win") {
    return (
      <span className="rounded bg-bg-3 px-1.5 py-0.5 font-mono text-[10px] uppercase text-ink-muted">
        {g.status.player} won
      </span>
    );
  }
  return (
    <span className="rounded bg-bg-3 px-1.5 py-0.5 font-mono text-[10px] uppercase text-ink-muted">
      {g.kind}
    </span>
  );
}

function AiSetupModal({
  onCancel,
  onStart,
}: {
  onCancel: () => void;
  /** `side` is the *human* side; the engine takes the other one. */
  onStart: (humanSide: Player, depth: number) => void;
}) {
  const [side, setSide] = useState<Player>("black");
  const [depth, setDepth] = useState(4);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-bg-0/70 p-6"
      onClick={onCancel}
    >
      <div
        className="w-full max-w-sm rounded-lg border border-border bg-bg-1 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-border px-4 py-3">
          <h2 className="text-sm font-medium text-ink-strong">Play vs AI</h2>
          <button
            onClick={onCancel}
            aria-label="Close"
            className="text-ink-muted hover:text-ink-strong"
          >
            <X className="size-4" />
          </button>
        </div>

        <div className="space-y-4 p-4">
          <Field label="You play">
            <div className="flex gap-2">
              {(["black", "white"] as Player[]).map((p) => (
                <button
                  key={p}
                  onClick={() => setSide(p)}
                  className={cn(
                    "flex flex-1 items-center justify-center gap-2 rounded-md border px-3 py-2 text-sm transition-colors",
                    side === p
                      ? "border-accent bg-accent/15 text-accent"
                      : "border-border bg-bg-2 text-ink-strong hover:bg-bg-3",
                  )}
                >
                  <Circle
                    className={cn(
                      "size-3",
                      p === "black"
                        ? "fill-stone-black stroke-0"
                        : "fill-stone-white stroke-0",
                    )}
                  />
                  {p === "black" ? "Black (opens)" : "White"}
                </button>
              ))}
            </div>
          </Field>

          <Field label={`AI depth: ${depth}`}>
            <input
              type="range"
              min={1}
              max={6}
              value={depth}
              onChange={(e) => setDepth(parseInt(e.target.value, 10))}
              className="w-full accent-accent"
            />
            <div className="flex justify-between text-[10px] text-ink-muted">
              <span>fast (1)</span>
              <span>strong (6)</span>
            </div>
          </Field>
        </div>

        <div className="flex justify-end gap-2 border-t border-border bg-bg-1 px-4 py-3">
          <button
            onClick={onCancel}
            className="rounded-md bg-bg-2 px-3 py-1.5 text-xs text-ink-strong hover:bg-bg-3"
          >
            Cancel
          </button>
          <button
            onClick={() => onStart(side, depth)}
            className="rounded-md bg-accent px-3 py-1.5 text-xs font-medium text-bg-0 hover:bg-accent/85"
          >
            Start
          </button>
        </div>
      </div>
    </div>
  );
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1.5">
      <div className="text-xs font-medium text-ink-muted">{label}</div>
      {children}
    </div>
  );
}
