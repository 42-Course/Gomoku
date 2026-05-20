import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Copy, User } from "lucide-react";
import { ensureIdentity, setDisplayName } from "@/storage/identity";
import { listGames, queryKeys } from "@/api/client";
import { cn } from "@/lib/cn";

const IDENTITY_KEY = ["identity", "self"] as const;

/**
 * Local profile. No login involved — this is the "who I am on this
 * device" record from IndexedDB. Display name is editable; the UUID is
 * the stable handle that future signed records will be tied to.
 */
export function Profile() {
  const qc = useQueryClient();
  const { data: identity } = useQuery({
    queryKey: IDENTITY_KEY,
    queryFn: ensureIdentity,
  });
  const { data: games } = useQuery({
    queryKey: queryKeys.games,
    queryFn: listGames,
  });

  const [draft, setDraft] = useState("");
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (identity) setDraft(identity.displayName);
  }, [identity?.displayName]);

  if (!identity) {
    return <div className="p-10 text-sm text-ink-muted">Loading profile…</div>;
  }

  const localCount = (games ?? []).filter((g) => g.kind === "local").length;
  const fixtureCount = (games ?? []).filter((g) => g.kind === "fixture").length;
  const dirty = draft.trim() !== identity.displayName && draft.trim() !== "";

  const save = async () => {
    const next = await setDisplayName(draft);
    qc.setQueryData(IDENTITY_KEY, next);
  };

  const copy = async () => {
    await navigator.clipboard.writeText(identity.uuid);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="mx-auto max-w-2xl px-8 py-10">
      <div className="mb-8 flex items-center gap-3">
        <div className="grid size-10 place-items-center rounded-md bg-accent/15 text-accent">
          <User className="size-5" />
        </div>
        <div>
          <h1 className="text-2xl font-semibold text-ink-strong">Profile</h1>
          <p className="text-sm text-ink-muted">
            Local-only. Signed records and accounts come later.
          </p>
        </div>
      </div>

      <div className="flex flex-col gap-4">
        <Field label="Display name">
          <div className="flex gap-2">
            <input
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              maxLength={48}
              className={cn(
                "flex-1 rounded-md border border-border bg-bg-1 px-3 py-2 text-sm text-ink-strong",
                "outline-none focus:border-accent",
              )}
            />
            <button
              onClick={save}
              disabled={!dirty}
              className={cn(
                "rounded-md px-3 py-2 text-xs font-medium transition-colors",
                dirty
                  ? "bg-accent text-bg-0 hover:bg-accent/85"
                  : "bg-bg-2 text-ink-muted",
              )}
            >
              Save
            </button>
          </div>
        </Field>

        <Field label="Local ID">
          <div className="flex items-center gap-2">
            <code className="flex-1 truncate rounded-md border border-border bg-bg-1 px-3 py-2 font-mono text-xs text-ink-strong">
              {identity.uuid}
            </code>
            <button
              onClick={copy}
              className="rounded-md border border-border bg-bg-1 px-2 py-2 text-ink-muted hover:bg-bg-2 hover:text-ink-strong"
              aria-label="Copy ID"
            >
              {copied ? <Check className="size-4" /> : <Copy className="size-4" />}
            </button>
          </div>
          <div className="mt-1 text-[11px] text-ink-muted">
            Generated on first load. Survives page reloads, lives in your
            browser only.
          </div>
        </Field>

        <Field label="Library">
          <div className="grid grid-cols-2 gap-2">
            <Stat label="Saved games" value={localCount.toString()} />
            <Stat label="Fixture games" value={fixtureCount.toString()} />
          </div>
        </Field>

        <Field label="Created">
          <span className="font-mono text-xs text-ink-muted">
            {new Date(identity.createdAt).toLocaleString()}
          </span>
        </Field>
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
    <div className="flex flex-col gap-1.5">
      <span className="text-[11px] font-medium uppercase tracking-wider text-ink-muted">
        {label}
      </span>
      {children}
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md border border-border bg-bg-1 p-3">
      <div className="text-[11px] uppercase tracking-wider text-ink-muted">
        {label}
      </div>
      <div className="font-mono text-lg text-ink-strong">{value}</div>
    </div>
  );
}
