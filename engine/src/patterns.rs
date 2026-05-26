//! Bitwise pattern detection on a single packed line.
//!
//! A "line" is one row, column, or diagonal of the board, packed as two
//! `u32`s, `me` (the bits of the player we're scoring) and `opp` (their
//! opponent). Bit `i` of each word corresponds to cell `i` along the line.
//! `len` says how many of the low bits are part of the line; the rest are
//! zero in both words and therefore behave as "off-board" (a wall: not me,
//! not opp, not empty).
//!
//! That convention is the whole reason this module is short: a pattern
//! requiring an *empty* cell at an endpoint won't match against a board
//! edge, the bit there is 0 in `empty()`, and a pattern requiring our
//! stone won't see one off-board either. Edges fall out for free.
//!
//! # Public API
//!
//! Packed-line (`u32`) helpers:
//!
//! - [`count_patterns`] — tally every maximal run on a line by length and
//!   openness.
//! - [`has_free_three`] — yes/no check for the four free-three shapes,
//!   used by the double-three rule.
//! - [`PatternCounts`] — the tally type produced by [`count_patterns`].
//!
//! Whole-board ([`BitBoard`]) helpers:
//!
//! - [`five_mask`] — starting cells of every contiguous five along a
//!   direction.
//! - [`expand_five`] — expand five-start bits into full five-cell masks.
//! - [`capturable_mask`] — every player stone that is part of a
//!   capturable pair in any direction.
//! - [`has_stable_five`] — does the player have a five whose stones
//!   can't all be broken by a capture?

#![allow(dead_code)]

use crate::game::Direction;
use crate::board::BitBoard;

/// Tally of distinct max-length runs found on one line for one player.
///
/// "Max-length" means each entry counts a run by its longest extent: a
/// 5-run is *not* also counted as several 4-runs.
///
/// "Open" runs have empty cells on both sides; "closed" runs have an
/// opponent stone or the board edge on exactly one side.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PatternCounts {
    pub stable_five: u32,
    pub unstable_five: u32,
    /// Runs of 5 or more in a row.
    // pub fives: u32,
    /// 4-runs with empty cells on both sides.
    pub open_four: u32,
    /// 4-runs with one side blocked.
    pub closed_four: u32,
    /// 3-runs with empty cells on both sides.
    pub open_three: u32,
    /// 3-runs with one side blocked.
    pub closed_three: u32,
    /// 2-runs with empty cells on both sides.
    pub open_two: u32,
    /// 2-runs with one side blocked.
    pub closed_two: u32,
}

impl PatternCounts {
    /// Add the counts in `rhs` to `self` field-by-field.
    pub fn add(&mut self, rhs: &PatternCounts) {

        self.stable_five += rhs.stable_five;
        self.unstable_five += rhs.unstable_five;
        // self.fives += rhs.fives;
        self.open_four += rhs.open_four;
        self.closed_four += rhs.closed_four;
        self.open_three += rhs.open_three;
        self.closed_three += rhs.closed_three;
        self.open_two += rhs.open_two;
        self.closed_two += rhs.closed_two;
    }
}

#[inline]
fn line_mask(len: u32) -> u32 {
    if len >= 32 { u32::MAX } else { (1u32 << len) - 1 }
}

/// Bits set at position `i` iff a maximal run of `me` of *exactly* length
/// `k` starts at `i`. "Maximal" = the cell before (`i-1`) and after
/// (`i+k`) are not `me`, where off-the-line counts as not-me too.
#[inline]
fn run_starts(m: u32, k: u32, mask: u32) -> u32 {
    let mut consecutive = m & mask;
    for s in 1..k {
        consecutive &= m >> s;
    }
    consecutive &= mask;

    let not_left = !(m << 1);
    let not_right = !(m >> k);
    consecutive & not_left & not_right & mask
}

pub fn split_runs(
    me: BitBoard,
    opp: BitBoard,
    dir: Direction,
    k: usize,
) -> (BitBoard, BitBoard) {
    let empty = !(me | opp);

    //
    // Exact-k maximal runs.
    //
    let mut run = me;

    for _ in 1..k {
        run &= dir.forward(run);
    }

    let not_left =
        !dir.backward(me);

    let not_right =
        !dir.forward_n(me, k);

    run &=
        not_left &
        not_right;

    //
    // Openness classification.
    //
    let left_open =
        dir.backward(empty);

    let right_open =
        dir.forward_n(empty, k);

    let open =
        run &
        left_open &
        right_open;

    let closed =
        run &
        (left_open ^ right_open);

    (open, closed)
}

pub fn classify_fives(me: BitBoard, dir: Direction, capturable: BitBoard) -> (u32, u32) {
    let mut stable = 0;
    let mut unstable = 0;

    let mut starts = five_mask(me, dir);

    if !starts.any() {
        return (0, 0);
    }

    let mut killed = capturable;
    let mut acc = capturable;

    for _ in 0..4 {
        acc = dir.backward(acc);
        killed |= acc;
    }

    let stable_starts = starts & !killed;
    let unstable_starts = starts & killed;

    while starts.any() {
        // Start one connected 5+ group.
        let mut current = starts.pop_lsb();

        let mut is_stable =
            (current & stable_starts).any();

        loop {
            // Adjacent five-starts belong to the
            // same overline structure.
            let next =
                dir.forward(current) & starts;

            if !next.any() {
                break;
            }

            current |= next;
            starts &= !next;

            if (next & stable_starts).any() {
                is_stable = true;
            }
        }

        starts &= !current;

        if is_stable {
            stable += 1;
        } else {
            unstable += 1;
        }
    }
    (stable, unstable)
}

/// Walk one packed line and tally every distinct run by length and openness.
///
/// Each maximal run contributes to exactly one bucket — a 5-run is a
/// `fives`, *not* additionally a `closed_four` plus a `closed_three`. Runs
/// shorter than two cells are ignored.
///
/// # Arguments
///
/// - `me`  — bits of the player whose patterns we're tallying.
/// - `opp` — bits of the opponent.
/// - `len` — number of cells in the packed line (`<= 19`).
///
/// # Examples
///
/// ```ignore
/// // ".XXXX." has one open four.
/// let me  = 0b011110;
/// let opp = 0;
/// let counts = count_patterns(me, opp, 6);
/// assert_eq!(counts.open_four, 1);
/// ```

pub fn count_patterns_new(
    me: BitBoard,
    opp: BitBoard,
) -> PatternCounts {
    let capturable =
        capturable_mask(me, opp);

    let mut res = PatternCounts::default();

    for dir in Direction::all() {
        let (stable_five, unstable_five) = classify_fives(me, dir, capturable);
        res.stable_five += stable_five;
        res.unstable_five += unstable_five;
        
        let (open_four, closed_four) = split_runs(me, opp, dir, 4);
        let (open_three, closed_three) = split_runs(me, opp, dir, 3);
        let (open_two, closed_two) = split_runs(me, opp, dir, 2);

        res.open_four += open_four.count_ones();
        res.closed_four += closed_four.count_ones();
        res.open_three += open_three.count_ones();
        res.closed_three += closed_three.count_ones();
        res.open_two += open_two.count_ones();
        res.closed_two += closed_two.count_ones();
    }
    res
}
pub fn count_patterns(me: u32, opp: u32, len: u32) -> PatternCounts {
    let mask = line_mask(len);
    let m = me & mask;
    let o = opp & mask;
    let e = !(m | o) & mask;

    // Anything 5+ in a row counts as a five, a single bit is enough.
    let five_or_more = (m & (m >> 1) & (m >> 2) & (m >> 3) & (m >> 4)) & mask;
    let fives = five_or_more.count_ones();

    let four = run_starts(m, 4, mask);
    let three = run_starts(m, 3, mask);
    let two = run_starts(m, 2, mask);

    let split = |run: u32, k: u32| -> (u32, u32) {
        // bit i of (e << 1) holds e[i-1]; bit i of (e >> k) holds e[i+k].
        let left_open = (e << 1) & mask;
        let right_open = (e >> k) & mask;
        let both_open = run & left_open & right_open;
        let one_open = run & (left_open ^ right_open);
        (both_open.count_ones(), one_open.count_ones())
    };

    let (open_four, closed_four) = split(four, 4);
    let (open_three, closed_three) = split(three, 3);
    let (open_two, closed_two) = split(two, 2);

    PatternCounts {
        stable_five: fives,
        unstable_five: 0,
        open_four,
        closed_four,
        open_three,
        closed_three,
        open_two,
        closed_two,
    }
}

/// Does this packed line contain *any* free-three pattern for `me`?
///
/// Free threes are the patterns that turn into an open four with one move:
/// `.XXX..`, `..XXX.`, `.XX.X.`, `.X.XX.` (each in a 6-cell window with
/// empty endpoints). The Gomoku double-three rule needs to know whether
/// playing a stone has *created* one of these along a given direction.
pub fn has_free_three(me: u32, opp: u32, len: u32) -> bool {
    let mask = line_mask(len);
    let m = me & mask;
    let o = opp & mask;
    let e = !(m | o) & mask;

    // Each pattern is a 6-cell window. Bit i of the result means the
    // pattern starts at position i, so we need every shift to align.
    let solid_l = e & (m >> 1) & (m >> 2) & (m >> 3) & (e >> 4) & (e >> 5);
    let solid_r = e & (e >> 1) & (m >> 2) & (m >> 3) & (m >> 4) & (e >> 5);
    let split_l = e & (m >> 1) & (m >> 2) & (e >> 3) & (m >> 4) & (e >> 5);
    let split_r = e & (m >> 1) & (e >> 2) & (m >> 3) & (m >> 4) & (e >> 5);

    ((solid_l | solid_r | split_l | split_r) & mask) != 0
}
pub fn free_three_mask(
    me: BitBoard,
    opp: BitBoard,
    dir: Direction,
) -> BitBoard {
    let empty = !(me | opp);

    //
    // Pattern windows:
    //
    // .XXX..
    // ..XXX.
    // .XX.X.
    // .X.XX.
    //
    // Returned bits are canonical starts of the
    // 6-cell pattern windows.
    //

    let solid_l =
        empty &
        dir.forward_n(me, 1) &
        dir.forward_n(me, 2) &
        dir.forward_n(me, 3) &
        dir.forward_n(empty, 4) &
        dir.forward_n(empty, 5);

    let solid_r =
        empty &
        dir.forward_n(empty, 1) &
        dir.forward_n(me, 2) &
        dir.forward_n(me, 3) &
        dir.forward_n(me, 4) &
        dir.forward_n(empty, 5);

    let split_l =
        empty &
        dir.forward_n(me, 1) &
        dir.forward_n(me, 2) &
        dir.forward_n(empty, 3) &
        dir.forward_n(me, 4) &
        dir.forward_n(empty, 5);

    let split_r =
        empty &
        dir.forward_n(me, 1) &
        dir.forward_n(empty, 2) &
        dir.forward_n(me, 3) &
        dir.forward_n(me, 4) &
        dir.forward_n(empty, 5);

    solid_l |
    solid_r |
    split_l |
    split_r
}

/// Returns the starting cells of every contiguous
/// five alignment in the given direction.
#[inline]
pub fn five_mask(
    bits: BitBoard,
    dir: Direction,
) -> BitBoard {
    let run2 = bits & dir.backward(bits);
    let run3 = run2 & dir.backward(run2);
    let run4 = run3 & dir.backward(run3);
    run4 & dir.backward(run4)
}

/// Expands five starting positions into full five-cell masks.
#[inline]
pub fn expand_five(
    starts: BitBoard,
    dir: Direction,
) -> BitBoard {
    let b = dir.forward(starts);
    let c = dir.forward(b);
    let d = dir.forward(c);
    let e = dir.forward(d);

    starts | b | c | d | e
}

/// Returns whether the player has a stable
/// five-in-a-row alignment.
///
/// A five is "stable" if no stone within its five cells can be
/// captured by the opponent. When a direction contains multiple
/// overlapping fives (e.g. a six-in-a-row contains two), each
/// candidate is checked individually — a five whose own five
/// cells don't intersect any capturable stone wins even if a
/// sibling five in the same direction is unstable.
pub fn has_stable_five(
    me: BitBoard,
    opp: BitBoard,
) -> bool {
    let capturable = capturable_mask(me, opp);

    for dir in Direction::all() {
        let starts = five_mask(me, dir);

        if !starts.any() {
            continue;
        }

        // Dilate `capturable` backward 0..=4 steps so that every
        // start position whose five-cell window covers a capturable
        // stone gets a bit set. The complement intersected with
        // `starts` is the set of stable five starts.
        let mut killed = capturable;
        let mut acc = capturable;
        for _ in 0..4 {
            acc = dir.backward(acc);
            killed |= acc;
        }

        if (starts & !killed).any() {
            return true;
        }
    }

    false
}

/// Returns all stones that are currently capturable by the opponent.
pub fn capturable_mask(
    me: BitBoard,
    opp: BitBoard,
) -> BitBoard {
    let empty = !(me | opp);

    let mut out = BitBoard::new();
    for dir in Direction::all() {
        out |= capturable_pairs_dir(
            me,
            opp,
            empty,
            dir,
        );
    }
    out
}

/// Returns all capturable pairs in one direction.
///
/// Each bit `i` in the result marks a player stone that is part
/// of a capturable pair along `dir`. The two arms below match the
/// pair against position `i` (the *forward* stone of the pair),
/// then `dir.backward(starts)` adds the *backward* stone of each
/// matched pair.
#[inline]
fn capturable_pairs_dir(
    player: BitBoard,
    opponent: BitBoard,
    empty: BitBoard,
    dir: Direction
) -> BitBoard {
    //
    // . X X O   (empty backward of pair, opponent forward of pair)
    //
    let left =
        player &
        dir.forward(player) &
        dir.forward(dir.forward(empty)) &
        dir.backward(opponent);

    //
    // O X X .   (opponent backward of pair, empty forward of pair)
    //
    let right =
        player &
        dir.forward(player) &
        dir.backward(empty) &
        dir.forward(dir.forward(opponent));

    let starts = left | right;

    starts | dir.backward(starts)
}

#[cfg(test)]
mod tests {
    use crate::game::Pos;
    use super::*;

    /// Build a line from a string of `X`/`O`/`.` characters, returning
    /// (me, opp, len). `X` is me.
    fn line(s: &str) -> (u32, u32, u32) {
        let mut me = 0u32;
        let mut opp = 0u32;
        for (i, c) in s.chars().enumerate() {
            match c {
                'X' => me |= 1 << i,
                'O' => opp |= 1 << i,
                '.' => {}
                other => panic!("bad cell '{other}' in line {s:?}"),
            }
        }
        (me, opp, s.chars().count() as u32)
    }

    // #[test]
    // fn five_in_a_row_is_a_five() {
    //     let (m, o, l) = line(".XXXXX.");
    //     let c = count_patterns(m, o, l);
    //     assert_eq!(c.fives, 1);
    //     // The 5-run isn't double-counted as smaller runs.
    //     assert_eq!(c.open_four, 0);
    //     assert_eq!(c.closed_four, 0);
    // }

    // #[test]
    // fn open_four_pattern() {
    //     let (m, o, l) = line("..XXXX..");
    //     let c = count_patterns(m, o, l);
    //     assert_eq!(c.open_four, 1);
    //     assert_eq!(c.closed_four, 0);
    // }

    // #[test]
    // fn closed_four_blocked_by_opponent() {
    //     let (m, o, l) = line("OXXXX..");
    //     let c = count_patterns(m, o, l);
    //     assert_eq!(c.open_four, 0);
    //     assert_eq!(c.closed_four, 1);
    // }

    // #[test]
    // fn closed_four_blocked_by_edge() {
    //     let (m, o, l) = line("XXXX..");
    //     let c = count_patterns(m, o, l);
    //     assert_eq!(c.closed_four, 1);
    //     assert_eq!(c.open_four, 0);
    // }

    // #[test]
    // fn open_three_and_open_two() {
    //     let (m, o, l) = line("..XX...XXX..");
    //     let c = count_patterns(m, o, l);
    //     assert_eq!(c.open_two, 1);
    //     assert_eq!(c.open_three, 1);
    // }

    #[test]
    fn solid_three_is_a_free_three() {
        let (m, o, l) = line("..XXX..");
        assert!(has_free_three(m, o, l));
    }

    #[test]
    fn split_three_is_a_free_three() {
        let (m, o, l) = line(".XX.X.");
        assert!(has_free_three(m, o, l));
    }

    #[test]
    fn closed_three_is_not_a_free_three() {
        let (m, o, l) = line("OXXX..");
        assert!(!has_free_three(m, o, l));
    }

    #[test]
    fn three_against_edge_is_not_a_free_three() {
        // No empty cell to the left, so neither solid nor split pattern fits.
        let (m, o, l) = line("XXX...");
        assert!(!has_free_three(m, o, l));
    }

    fn bb(coords: &[(usize, usize)]) -> BitBoard {
        let mut out = BitBoard::new();

        for &(x, y) in coords {
            out.place_stone(Pos::from_xy(x, y));
        }

        out
    }

    #[test]
    fn detects_horizontal_five() {
        let stones = bb(&[
            (0, 0),
            (1, 0),
            (2, 0),
            (3, 0),
            (4, 0),
        ]);

        assert!(
            five_mask(
                stones,
                Direction::Horizontal
            ).any()
        );
    }

    #[test]
    fn detects_vertical_five() {
        let stones = bb(&[
            (0, 0),
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 4),
        ]);

        assert!(
            five_mask(
                stones,
                Direction::Vertical
            ).any()
        );
    }

    #[test]
    fn detects_diagonal_five() {
        let stones = bb(&[
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
        ]);

        assert!(
            five_mask(
                stones,
                Direction::Diagonal
            ).any()
        );
    }

    #[test]
    fn detects_antidiagonal_five() {
        let stones = bb(&[
            (0, 4),
            (1, 3),
            (2, 2),
            (3, 1),
            (4, 0),
        ]);

        assert!(
            five_mask(
                stones,
                Direction::AntiDiagonal
            ).any()
        );
    }

    #[test]
    fn horizontal_does_not_wrap() {
        let stones = bb(&[
            (18, 0),
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 1),
        ]);

        assert!(
            !five_mask(
                stones,
                Direction::Horizontal
            ).any()
        );
    }

    #[test]
    fn detects_capturable_pair_horizontal() {
        //
        // O X X .
        //
        let me = bb(&[
            (1, 0),
            (2, 0),
        ]);

        let opp = bb(&[
            (0, 0),
        ]);

        let capturable =
            capturable_mask(me, opp);

        assert!(
            capturable.is_occupied(
                Pos::from_xy(1, 0)
            )
        );

        assert!(
            capturable.is_occupied(
                Pos::from_xy(2, 0)
            )
        );
    }

    #[test]
    fn capturable_oxx_dot_blocked_by_extra_stone() {
        // O X X . X
        //
        // Opponent playing at (3, 0) makes O X X O and captures
        // (1, 0) and (2, 0). The extra X at (4, 0) must not hide
        // the capture from `capturable_mask`.
        let me = bb(&[(1, 0), (2, 0), (4, 0)]);
        let opp = bb(&[(0, 0)]);
        let capturable = capturable_mask(me, opp);
        assert!(capturable.is_occupied(Pos::from_xy(1, 0)));
        assert!(capturable.is_occupied(Pos::from_xy(2, 0)));
    }

    #[test]
    fn capturable_dot_xx_o_blocked_by_extra_stone() {
        // X . X X O
        //
        // Mirror of the above: empty at (1, 0), pair at (2, 0)-(3, 0),
        // O at (4, 0), and an extra X at (0, 0). The pair is capturable
        // by playing at (1, 0).
        let me = bb(&[(0, 0), (2, 0), (3, 0)]);
        let opp = bb(&[(4, 0)]);
        let capturable = capturable_mask(me, opp);
        assert!(capturable.is_occupied(Pos::from_xy(2, 0)));
        assert!(capturable.is_occupied(Pos::from_xy(3, 0)));
    }

    #[test]
    fn six_in_a_row_with_one_capturable_end_is_stable() {
        //   .  .  .  .  .  .
        //   X  X  X  X  X  X   <- row 1
        //   X  .  .  .  .  .
        //   O  .  .  .  .  .
        //
        // The five at (1, 1)-(5, 1) does not contain (0, 1) or
        // (0, 2), so it is stable even though (0, 1) is part of a
        // capturable vertical pair.
        let me = bb(&[
            (0, 1), (1, 1), (2, 1), (3, 1), (4, 1), (5, 1),
            (0, 2),
        ]);
        let opp = bb(&[(0, 3)]);
        assert!(has_stable_five(me, opp));
    }

    #[test]
    fn stable_five_detected() {
        let me = bb(&[
            (0, 0),
            (1, 0),
            (2, 0),
            (3, 0),
            (4, 0),
        ]);

        let opp = BitBoard::new();

        assert!(
            has_stable_five(me, opp)
        );
    }

    #[test]
    fn unstable_five_rejected() {
        // .
        // X X X X X
        // X
        // O
        let me = bb(&[
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 1),
            (4, 1),
            (0, 2)
        ]);

        let opp = bb(&[
            (0, 3),
        ]);

        assert!(
            !has_stable_five(me, opp)
        );
    }

    #[test]
    fn unstable_five_rejected_diagonal() {
        // .
        // X X X X X
        // . . X
        // . . . O
        let me = bb(&[
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 1),
            (4, 1),
            (2, 2)
        ]);

        let opp = bb(&[
            (3, 3),
        ]);

        assert!(
            !has_stable_five(me, opp)
        );
    }

    #[test]
    fn stable_five_when_diagonal_pair_is_double_flanked() {
        // O
        // X X X X X
        // . . X
        // . . . O
        //
        // The diagonal pair (1, 1)-(2, 2) is bracketed by O on BOTH
        // sides ((0, 0) and (3, 3)), so the opponent has no empty
        // cell to play into to complete a capture. The pair is not
        // currently capturable, so the horizontal five at row 1 is
        // stable.
        let me = bb(&[
            (0, 1),
            (1, 1),
            (2, 1),
            (3, 1),
            (4, 1),
            (2, 2)
        ]);

        let opp = bb(&[
            (0, 0),
            (3, 3),
        ]);

        assert!(
            has_stable_five(me, opp)
        );
    }
}
