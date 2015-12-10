use conv::TryFrom;
use std::collections::btree_map::BTreeMap;

type Coordinate = (i8, i8);

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum Stone {
    Black,
    White
}

custom_derive! {
    #[derive(Debug, Copy, Clone, TryFrom(usize))]
    pub enum Size {
        Nine = 9,
        Thirteen = 13,
        Seventeen = 17,
        Nineteen = 19
    }
}

pub struct Game {
    id: u64,
    board: BTreeMap<Coordinate, Stone>,
    size: Size,
    turn: Stone
}

pub fn new(size: Size) -> Game {
    Game{
        id: 0,
        board: BTreeMap::new(),
        size: size,
        turn: Stone::Black}
}

pub fn parse(board_str: &str, turn: Stone) -> Option<Game> {
    let mut board = BTreeMap::new();
    let lines : Vec<&str> = board_str.trim().split("\n").collect();

    if lines.len() < (Size::Nine as usize) || lines.len() > (Size::Nineteen as usize) {
        return None;
    }

    let line_length = lines[0].trim().len();
    if line_length != lines.len() {
        return None;
    }

    let size_result = Size::try_from(line_length);
    let size = match size_result {
        Ok(size) => size,
        _ => return None,
    };

    for (y, line) in lines.iter().enumerate() {
        for (x, tile) in line.chars().enumerate() {
            match tile {
                'b' => { board.insert((x as i8, y as i8), Stone::Black); },
                'w' => { board.insert((x as i8, y as i8), Stone::White); },
                _ => (),
            }
        }
    }

    Some(Game{
        id: 0,
        board: board,
        size: size,
        turn: turn})
}

impl Game {
    pub fn turn(&self) -> Stone {
        self.turn
    }

    fn valid_coordinate(&self, (x, y): Coordinate) -> bool {
        x >= 0 && x < self.size as i8 && y >= 0 && y < self.size as i8
    }

    fn free_tile(&self, position: Coordinate) -> bool {
        !self.board.contains_key(&position)
    }

    fn neighbour_count(&self, (x, y): Coordinate, stone: Stone) -> usize {
        let adjacent_tiles = [(x, y - 1), (x + 1, y), (x, y + 1), (x - 1, y)];
        adjacent_tiles.iter()
            .map(|coordinate| { self.board.get(coordinate) })
            .filter(|tile| tile.is_some())
            .count()
    }

    fn is_suicide(&self, (x, y): Coordinate, stone: Stone) -> bool {
        self.neighbour_count((x, y), stone) > 0
    }

    fn can_play(&self, position: Coordinate, stone: Stone) -> bool {
        self.turn == stone &&
            !self.is_suicide(position, stone) &&
            self.valid_coordinate(position) &&
            self.free_tile(position)
    }

    fn advance_turn(&mut self) {
        match self.turn {
            Stone::Black => self.turn = Stone::White,
            Stone::White => self.turn = Stone::Black,
        }
    }

    pub fn play_stone(&mut self, position: Coordinate, stone: Stone) -> bool {
        if self.can_play(position, stone) {
            self.board.insert(position, stone);
            self.advance_turn();
            true
        } else {
            false
        }
    }

    pub fn has_stone(&self, position: Coordinate) -> bool {
        self.board.contains_key(&position)
    }

    pub fn stones(&self) -> usize {
        self.board.len()
    }

    pub fn player_stones(&self, stone: Stone) -> usize {
        self.board.iter().filter(|&(_, piece)| *piece == stone).count()
    }

    pub fn player_score(&self, stone: Stone) -> usize {
        self.board.iter().filter(|&(_, piece)| *piece == stone).count()
    }

    pub fn winner(&self) -> Stone {
        Stone::Black
    }
}

#[test]
fn test_new() {
    let game = new(Size::Nine);
    assert_eq!(game.id, 0);
    assert_eq!(Stone::Black, game.turn());
    assert_eq!(false, game.has_stone((0, 0)));
}

#[test]
fn test_play_stone() {
    let mut game = new(Size::Nine);
    assert_eq!(false, game.has_stone((0, 0)));

    game.play_stone((0, 0), Stone::Black);
    assert_eq!(true, game.has_stone((0, 0)));
    assert_eq!(1, game.stones());
    assert_eq!(1, game.player_stones(Stone::Black));
    assert_eq!(0, game.player_stones(Stone::White));
}

#[test]
fn test_play_stone_switches_players() {
    let mut game = new(Size::Nine);
    assert_eq!(true, game.play_stone((0, 0), Stone::Black));
    assert_eq!(false, game.play_stone((1, 0), Stone::Black));
}

#[test]
fn test_play_stone_rejects_invalid_plays() {
    let mut game = new(Size::Nine);
    assert_eq!(false, game.play_stone((10, 0), Stone::Black));
    assert_eq!(false, game.play_stone((10, 10), Stone::Black));
    assert_eq!(false, game.play_stone((0, 10), Stone::Black));
}

#[test]
fn test_play_stone_rejects_duplicate_plays() {
    let mut game = new(Size::Nine);
    assert_eq!(true, game.play_stone((0, 0), Stone::Black));
    assert_eq!(false, game.play_stone((0, 0), Stone::White));
}

#[test]
fn test_parse() {
    let game = parse("
.b.......
b........
.........
.........
.........
.........
.........
.........
.........", Stone::Black).unwrap();

    assert_eq!(false, game.has_stone((0, 0)));
    assert_eq!(true, game.has_stone((1, 0)));
    assert_eq!(true, game.has_stone((0, 1)));
    assert_eq!(false, game.has_stone((1, 1)));
}

#[test]
fn test_play_stone_no_liberties() {
    let mut game = parse("
.b.....b.
b.bbbbbb.
.b....bbb
..bbb.b..
.....b...
.........
.........
b.......b
.b.....b.", Stone::White).unwrap();

    assert_eq!(false, game.play_stone((0, 0), Stone::White));
    assert_eq!(false, game.play_stone((1, 1), Stone::White));
    assert_eq!(false, game.play_stone((1, 2), Stone::White));
    assert_eq!(false, game.play_stone((9, 0), Stone::White));
    assert_eq!(false, game.play_stone((9, 9), Stone::White));
}
