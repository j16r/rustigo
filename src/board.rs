use std::collections::btree_map::BTreeMap;

type Coordinate = (u8, u8);

#[derive(Eq, PartialEq)]
enum Stone {
    Black,
    White
}

enum Size {
    Nine,
    Thirteen,
    Seventeen,
    Nineteen
}

pub struct Game {
    id: u64,
    stones: BTreeMap<Coordinate, Stone>,
    size: Size,
}

pub fn new() -> Game {
    Game{id: 0, stones: BTreeMap::new(), size: Size::Nineteen}
}

impl Game {
    pub fn play_stone(&mut self, stone: Stone, position: Coordinate) {
        self.stones.insert(position, stone);
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
}

#[test]
fn test_new() {
    let game = new();
    assert_eq!(game.id, 0);
    assert_eq!(false, game.has_stone((0, 0)));
}

#[test]
fn test_play_stone() {
    let mut game = new();
    assert_eq!(false, game.has_stone((0, 0)));

    game.play_stone(Stone::Black, (0, 0));
    assert_eq!(true, game.has_stone((0, 0)));
    assert_eq!(1, game.stones());
    assert_eq!(1, game.player_stones(Stone::Black));
    assert_eq!(0, game.player_stones(Stone::White));
}
