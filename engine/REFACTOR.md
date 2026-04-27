# Refactor: bitmap-driven pattern detection

This branch (`pulga/min-max-abp`) merged the `feature/bitmap` work from
`main` into the negamax + alpha-beta search and pulled every
pattern-matching codepath onto a single bitmap-backed implementation.
The two pieces that used to walk the board cell-by-cell now run as
bit-shifts on `u32` words.

## What used to live where

Before:

| Concern                     | Location                                  | Shape                                           |
| --------------------------- | ----------------------------------------- | ----------------------------------------------- |
| Search                      | `src/ai.rs`                               | Negamax + observer (this branch, pre-merge)     |
| Heuristic eval              | `src/ai/eval.rs` (from `feature/bitmap`)  | Stub: TODOs, called a non-existent `is_win_for` |
| Double-three rule           | `src/game.rs::is_free_three`              | Walk 9 cells, four hand-rolled `match` arms     |
| Storage                     | `src/board.rs::Board` (from `feature/bitmap`) | Two `BitBoard`s (one per player), 6×u64 each    |

After:

```
src/
├── ai/
│   ├── mod.rs        — re-exports
│   ├── search.rs     — negamax + alpha-beta, observer-based search tree
│   └── eval.rs       — heuristic; uses patterns::count_patterns
├── board.rs          — Board + BitBoard, plus pack_line / for_each_line
├── game.rs           — rules; is_free_three now calls patterns::has_free_three
└── patterns.rs       — the one place that knows what a "three" looks like
```

## How pattern detection actually works now

A *line* is one row, column, diagonal, or anti-diagonal of the board.
Lines are at most 19 cells long, so each line packs into two `u32`s:

- `me`  — bit `i` set when cell `i` along the line holds our stones
- `opp` — bit `i` set when it holds the opponent's
- `len` — how many low bits are part of the line; the rest are zero

The trick is that off-the-line cells are zero in both masks. They are
not "me", not "opp", and *not "empty"* — so a pattern requiring an
empty cell at an endpoint (`.XXXX.`) cannot match against a board edge.
Edge handling falls out for free; we never need a special case.

`Board::pack_line(x, y, dx, dy, max_len, player)` walks one direction
and produces `(me, opp, len)`. `Board::for_each_line(player, min_len, f)`
visits every distinct line on the board and calls `f` with each.

`patterns::count_patterns(me, opp, len) -> PatternCounts` recognizes:

- 5+ in a row
- Open / closed runs of length 4, 3, 2

Each run is counted at its *maximal* length: a 5-run is a five, not
also a four-and-a-three. The math:

```rust
// k consecutive `me` bits starting at position i
let mut consecutive = m;
for s in 1..k { consecutive &= m >> s; }

// "Maximal" — bit i-1 isn't me (or is off-board), bit i+k isn't either
let not_left  = !(m << 1);
let not_right = !(m >> k);
let starts    = consecutive & not_left & not_right & line_mask(len);
```

Open vs. closed for those starts comes from the empty-mask shifted by
the corresponding offsets. The whole evaluation walks every line on
the board *once*, so the per-call cost is roughly linear in the number
of stones rather than quadratic in the cell count.

## Free-three rule, same detector

`patterns::has_free_three(me, opp, len)` checks the four 6-cell
patterns that turn into an open four with one move:

```
.XXX..   ..XXX.   .XX.X.   .X.XX.
```

`game.rs::is_free_three` now packs the 9 cells around the played stone
into bits and asks the detector. The previous implementation had a
subtle alignment bug at the board edge — when a cell was off-board it
was *skipped* but the rest of the line shifted left, so window indices
no longer matched the played stone. The bitmap version drops off-board
cells from the packed bits, and because they're absent (not "empty"),
the patterns simply cannot match against the edge.

The win is consistency: the rule check and the evaluator now share one
definition of what "a three" is. They cannot drift apart.

## Search wiring

`ai::search::negamax` is unchanged in shape:

- Generic `Observer` trait with two impls — `NoopObserver` (compiled
  away) drives `best_move`, and `TreeObserver` records the visited
  tree for `best_move_verbose` (visualizer hook).
- Negamax convention: scores are from the side-to-move's perspective;
  a winning terminal returns `-(WIN_SCORE + depth)` so faster mates
  outrank slower ones.

Eval is the only thing that changed: it used to be a stub returning
`0`. It now scores from `score_player(board, me) - score_player(board, opp)`
plus `capture_diff * CAPTURE_PAIR`.

## Tests

28 passing as of this commit. Worth flagging:

- `patterns::tests` cover the bitmap detector directly with hand-typed
  lines like `"..XXXX.."` so failures point at the math, not the
  caller.
- `ai::eval::tests::open_three_outscores_open_two` is the contract
  between search and eval: the same side-to-move under more pressure
  must score worse.
- `ai::search::tests::blocks_an_immediate_win` is the integration
  test — White must take the only square that prevents Black from
  finishing five-in-a-row next ply.

## Adding a new pattern

If you want to recognize, say, a "double four" or a swap-2 opening
shape:

1. Pick the bit pattern you want to find. Express it as: `me` bits at
   positions `Mᵢ`, `empty` bits at positions `Eᵢ`, and `opp` bits at
   positions `Oᵢ`.
2. Translate to a single bitwise expression by shifting each input
   mask: `(m & (m >> 1) & ...) & (e >> k) & (!o >> j) & line_mask(len)`.
3. Add a counter to `PatternCounts` and a constant to `eval.rs` if it
   feeds the heuristic, or call it from `game.rs` if it's a rule.
4. Test it directly with `line("...")` helpers in
   `patterns::tests` before exposing it.

## Things deliberately not done here

- No iterative deepening, no transposition table, no move ordering.
  The search is correct but naive; ordering moves by `evaluate(after
  the move)` would help a lot.
- `pack_line` still uses `cell_at` per step. The further win is
  pulling rows/columns/diagonals out of the underlying `u64` words
  with shift+mask, but that requires a dedicated layout (or computed
  shift offsets) and was out of scope for this refactor.
- Capture detection in `apply_captures` still walks cells. A bitmap
  version would mask the four trios and AND them against the
  opponent's bitboard — same idea, smaller payoff because captures
  fire once per move, not once per evaluated node.
