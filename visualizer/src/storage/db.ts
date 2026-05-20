/**
 * Single IndexedDB connection used by every storage module.
 *
 * Migrations are numbered. Adding a new object store or index means
 * bumping `DB_VERSION` and adding a `if (oldVersion < N)` block — never
 * destructively modify existing stores in place.
 */

const DB_NAME = "gomoku";
const DB_VERSION = 1;

let cached: Promise<IDBDatabase> | null = null;

export function openDb(): Promise<IDBDatabase> {
  if (cached) return cached;
  cached = new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onerror = () => reject(req.error ?? new Error("IndexedDB open failed"));
    req.onsuccess = () => resolve(req.result);
    req.onupgradeneeded = (e) => {
      const db = req.result;
      const oldVersion = e.oldVersion;
      if (oldVersion < 1) {
        // Saved games keyed by id; secondary index on updatedAt for recency.
        const games = db.createObjectStore("games", { keyPath: "id" });
        games.createIndex("updatedAt", "updatedAt", { unique: false });
        // Single-row identity store (key "self").
        db.createObjectStore("identity", { keyPath: "id" });
      }
    };
  });
  return cached;
}

/** Run a transaction and return its result, rejecting on tx error. */
export function tx<T>(
  storeNames: string | string[],
  mode: IDBTransactionMode,
  fn: (tx: IDBTransaction) => Promise<T> | T,
): Promise<T> {
  return openDb().then(
    (db) =>
      new Promise<T>((resolve, reject) => {
        const transaction = db.transaction(storeNames, mode);
        let result: T;
        Promise.resolve()
          .then(() => fn(transaction))
          .then((r) => {
            result = r;
          })
          .catch(reject);
        transaction.oncomplete = () => resolve(result);
        transaction.onerror = () =>
          reject(transaction.error ?? new Error("tx failed"));
        transaction.onabort = () =>
          reject(transaction.error ?? new Error("tx aborted"));
      }),
  );
}

/** Promise wrapper around an IDBRequest. */
export function req<T>(r: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error ?? new Error("idb request failed"));
  });
}
