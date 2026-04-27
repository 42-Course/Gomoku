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

#![allow(dead_code)]

/// Tally of distinct max-length runs found on one line for one player.
///
/// "Max-length" means each entry counts a run by its longest extent: a
/// 5-run is *not* also counted as several 4-runs.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PatternCounts {
    pub fives: u32,
    pub open_four: u32,
    pub closed_four: u32,
    pub open_three: u32,
    pub closed_three: u32,
    pub open_two: u32,
    pub closed_two: u32,
}

impl PatternCounts {
    pub fn add(&mut self, rhs: &PatternCounts) {
        self.fives += rhs.fives;
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

/// Walk one packed line and tally every distinct run by length and openness.
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
        fives,
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

#[cfg(test)]
mod tests {
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

    #[test]
    fn five_in_a_row_is_a_five() {
        let (m, o, l) = line(".XXXXX.");
        let c = count_patterns(m, o, l);
        assert_eq!(c.fives, 1);
        // The 5-run isn't double-counted as smaller runs.
        assert_eq!(c.open_four, 0);
        assert_eq!(c.closed_four, 0);
    }

    #[test]
    fn open_four_pattern() {
        let (m, o, l) = line("..XXXX..");
        let c = count_patterns(m, o, l);
        assert_eq!(c.open_four, 1);
        assert_eq!(c.closed_four, 0);
    }

    #[test]
    fn closed_four_blocked_by_opponent() {
        let (m, o, l) = line("OXXXX..");
        let c = count_patterns(m, o, l);
        assert_eq!(c.open_four, 0);
        assert_eq!(c.closed_four, 1);
    }

    #[test]
    fn closed_four_blocked_by_edge() {
        let (m, o, l) = line("XXXX..");
        let c = count_patterns(m, o, l);
        assert_eq!(c.closed_four, 1);
        assert_eq!(c.open_four, 0);
    }

    #[test]
    fn open_three_and_open_two() {
        let (m, o, l) = line("..XX...XXX..");
        let c = count_patterns(m, o, l);
        assert_eq!(c.open_two, 1);
        assert_eq!(c.open_three, 1);
    }

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
}
