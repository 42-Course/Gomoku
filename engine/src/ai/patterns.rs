use crate::board::{ BitBoard, Board, WORD_COUNT };
use crate::game::Player;
use crate::constants::BOARD_SIZE;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
    Five,
    OpenFour,
    ClosedFour,
    BrokenFour,
    OpenThree,
    BrokenThree,
}



pub struct PatternHit {
    pub start: usize,
    pub dir: Direction,
    pub kind: PatternKind,
}

#[derive(Clone, Copy)]
pub enum Direction {
    Horizontal,
    Vertical,
    DiagDown,
    DiagUp,
}

impl Direction {
    pub fn shift(self) -> usize {
        match self {
            Direction::Horizontal => 1,
            Direction::Vertical => BOARD_SIZE,
            Direction::DiagDown => BOARD_SIZE + 1,
            Direction::DiagUp => BOARD_SIZE - 1,
        }
    }
}

pub fn count_open_three(board: &Board, player: Player) -> u32 {
    // detect_patterns(board, player)
    //     .into_iter()
    //     .filter(|hit| hit.kind == PatternKind::OpenThree)
    //     .count() as u32

    detect_open_three_positions(board, player).len() as u32
}

fn open_three_a(my: &BitBoard, empty: &BitBoard, s: usize) -> BitBoard{
    // .XXX..
    let three = my.and(&my.shr(s)).and(&my.shr(s * 2));
    let left_empty = empty.shl(s);
    let right_empty = empty.shr(s * 3);
    let tail_empty = empty.shr(s * 4);

    let result = three
        .and(&left_empty)
        .and(&right_empty)
        .and(&tail_empty);

    result
}

fn open_three_b(my: &BitBoard, empty: &BitBoard, s: usize) -> BitBoard{
    // ..XXX.
    let three = my.and(&my.shr(s)).and(&my.shr(s * 2));
    let start_empty = empty.shl(s * 2);
    let left_empty = empty.shl(s);
    let right_empty = empty.shr(s * 4);

    let result = three
        .and(&start_empty)
        .and(&left_empty)
        .and(&right_empty);

    result
}

fn open_three_c(my: &BitBoard, empty: &BitBoard, s: usize) -> BitBoard{
    // .XX.X.
    let three = my.and(&my.shr(s)).and(&my.shr(s * 2));
    let left_empty = empty.shl(s);
    let right_empty = empty.shr(s * 3);
    let tail_empty = empty.shr(s * 4);

    let result = three
        .and(&left_empty)
        .and(&right_empty)
        .and(&tail_empty);

    result
}

fn open_three_d(my: &BitBoard, empty: &BitBoard, s: usize) -> BitBoard{
    // .X.XX.
    let three = my.and(&my.shr(s)).and(&my.shr(s * 2));
    let left_empty = empty.shl(s);
    let right_empty = empty.shr(s * 3);
    let tail_empty = empty.shr(s * 4);

    let result = three
        .and(&left_empty)
        .and(&right_empty)
        .and(&tail_empty);

    result
}

fn count_open_three_stride(board: &Board, player: Player, direction: Direction) -> u32 {
    let my = board.bits(player);
    let opp = board.bits(player.opponent());

    let occupied = my.or(opp);
    let empty = occupied.not();

    let s = direction.shift();
    open_three_a(my, &empty, s).popcount()

}

pub fn count_open_four(board: Board, player: Player) -> u32 {
    //_XXXX_ we can use compression trick

    //we first find two stones in a row and then shift >> * 2

    //Therefore we get 2 consequtive paris of two
}