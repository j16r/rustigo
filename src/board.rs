use std::collections::btree_map::BTreeMap;

type Coordinate = (u8, u8);

#[derive(Eq, PartialEq, Debug)]
pub enum Stone {
    Black,
    White
}

#[derive(Copy, Clone)]
pub enum Size {
    Nine = 9,
    Thirteen = 13,
    Seventeen = 17,
    Nineteen = 19
}

pub struct Game {
    id: u64,
    stones: BTreeMap<Coordinate, Stone>,
    size: Size,
}

pub fn new(size: Size) -> Game {
    Game{id: 0, stones: BTreeMap::new(), size: size}
}

impl Game {
    pub fn turn(&self) -> Stone {
        Stone::Black
    }

    fn valid_coordinate(&self, (x, y): Coordinate) -> bool {
        x < self.size as u8 && y < self.size as u8
    }

    pub fn play_stone(&mut self, stone: Stone, position: Coordinate) -> bool {
        if self.valid_coordinate(position) {
            self.stones.insert(position, stone);
            true
        } else {
            false
        }
    }

    pub fn has_stone(&self, position: Coordinate) -> bool {
        self.stones.contains_key(&position)
    }

    pub fn stones(&self) -> u32 {
        self.stones.len() as u32
    }

    pub fn player_stones(&self, stone: Stone) -> u32 {
        self.stones.iter().filter(|&(_, piece)| *piece == stone).count() as u32
    }

    pub fn player_score(&self, stone: Stone) -> u32 {
        self.stones.iter().filter(|&(_, piece)| *piece == stone).count() as u32
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

    game.play_stone(Stone::Black, (0, 0));
    assert_eq!(true, game.has_stone((0, 0)));
    assert_eq!(1, game.stones());
    assert_eq!(1, game.player_stones(Stone::Black));
    assert_eq!(0, game.player_stones(Stone::White));
}

#[test]
fn test_play_stone_rejects_invalid_plays() {
    let mut game = new(Size::Nine);
    assert_eq!(false, game.play_stone(Stone::Black, (10, 0)));
    assert_eq!(false, game.play_stone(Stone::Black, (10, 10)));
    assert_eq!(false, game.play_stone(Stone::Black, (0, 10)));
}
