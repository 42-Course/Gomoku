/**
 * Game persistence — saves user-played games into IndexedDB.
 *
 * Fixtures (the demo content under `api/fixtures.ts`) are *not* stored
 * here; they're code-resident and merged at the listing layer. Anything
 * the user actually plays goes through these functions.
 */

import type { Game } from "@/api/types";
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
