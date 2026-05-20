/**
 * Local identity — the user's "who am I on this device" record.
 *
 * Per DESIGN.md §6, this will eventually carry an Ed25519 keypair and
 * optionally bind to a server account. For now it's just a UUID and a
 * mutable display name, so the rest of the UI has something to render
 * on the Profile page and on saved games.
 */

import { req, tx } from "./db";

export interface Identity {
  id: "self";
  uuid: string;
  displayName: string;
  createdAt: string;
}

function newUuid(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return Array.from({ length: 16 }, () =>
    Math.floor(Math.random() * 256)
      .toString(16)
      .padStart(2, "0"),
  ).join("");
}

/** Read the local identity, creating it on first call. */
export async function ensureIdentity(): Promise<Identity> {
  const existing = await tx("identity", "readonly", (t) =>
    req(t.objectStore("identity").get("self")),
  );
  if (existing) return existing as Identity;

  const fresh: Identity = {
    id: "self",
    uuid: newUuid(),
    displayName: "You",
    createdAt: new Date().toISOString(),
  };
  await tx("identity", "readwrite", (t) =>
    req(t.objectStore("identity").put(fresh)),
  );
  return fresh;
}

/** Update the display name; returns the new identity. */
export async function setDisplayName(name: string): Promise<Identity> {
  const trimmed = name.trim();
  if (!trimmed) throw new Error("display name cannot be empty");
  return tx("identity", "readwrite", async (t) => {
    const store = t.objectStore("identity");
    const cur = (await req(store.get("self"))) as Identity | undefined;
    if (!cur) throw new Error("identity not initialized");
    const next: Identity = { ...cur, displayName: trimmed };
    await req(store.put(next));
    return next;
  });
}
