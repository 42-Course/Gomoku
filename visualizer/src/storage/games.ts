/**
 * Game persistence — saves user-played games into IndexedDB.
 *
 * Fixtures (the demo content under `api/fixtures.ts`) are *not* stored
 * here; they're code-resident and merged at the listing layer. Anything
 * the user actually plays goes through these functions.
 *
 * A Game record is created *the moment the user starts a game* (Home →
 * "Start"), with an empty `moves` array and `status.kind === "ongoing"`.
 * That way the Recent strip on Home can show in-progress games and the
 * user can navigate away and come back without losing anything.
 */

import type { Game, GameMode, Player } from "@/api/types";
import { ensureIdentity } from "./identity";
import { req, tx } from "./db";

export async function listLocalGames(): Promise<Game[]> {
  return tx("games", "readonly", async (t) => {
    const all = (await req(t.objectStore("games").getAll())) as Game[];
    all.sort((a, b) => (a.updatedAt < b.updatedAt ? 1 : -1));
    return all;
  });
}

export async function getLocalGame(id: string): Promise<Game | null> {
  return tx("games", "readonly", async (t) => {
    const g = (await req(t.objectStore("games").get(id))) as Game | undefined;
    return g ?? null;
  });
}

export async function saveLocalGame(game: Game): Promise<void> {
  if (game.kind !== "local") throw new Error("only local games can be saved");
  await tx("games", "readwrite", (t) => req(t.objectStore("games").put(game)));
}

export async function deleteLocalGame(id: string): Promise<void> {
  await tx("games", "readwrite", (t) =>
    req(t.objectStore("games").delete(id)),
  );
}

export interface NewGameSpec {
  mode: GameMode;
  aiDepth?: number;
  aiSide?: Player;
}

/**
 * Create a fresh ongoing Game record and persist it.
 *
 * Returns the new id; the caller typically navigates straight to
 * `/play?id=<id>` afterwards. Player names are filled from the local
 * identity (and "AI (depth N)" for whichever side the engine takes in
 * vs-AI mode).
 */
export async function createLocalGame(spec: NewGameSpec): Promise<string> {
  const me = await ensureIdentity();
  const id = `local_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;
  const aiLabel =
    spec.mode === "vsai" && spec.aiDepth != null
      ? `AI (depth ${spec.aiDepth})`
      : "AI";
  const black =
    spec.mode === "vsai" && spec.aiSide === "black" ? aiLabel : me.displayName;
  const white =
    spec.mode === "vsai" && spec.aiSide === "white" ? aiLabel : me.displayName;
  const now = new Date().toISOString();
  const game: Game = {
    id,
    kind: "local",
    mode: spec.mode,
    title:
      spec.mode === "vsai"
        ? `vs AI (depth ${spec.aiDepth ?? "?"})`
        : "Hot-seat",
    black,
    white,
    status: { kind: "ongoing" },
    moveCount: 0,
    moves: [],
    captures: { black: 0, white: 0 },
    createdAt: now,
    updatedAt: now,
    aiDepth: spec.mode === "vsai" ? spec.aiDepth : undefined,
    aiSide: spec.mode === "vsai" ? spec.aiSide : undefined,
  };
  await saveLocalGame(game);
  return id;
}
