use conv::TryFrom;
use std::collections::HashSet;
use std::collections::btree_map::BTreeMap;
use std::fmt;
use std::iter::Iterator;

pub type Coordinate = (i8, i8);

#[derive(Eq, PartialEq, Debug, Copy, Serialize, Deserialize, Clone)]
pub enum Stone {
    Black,
    White
}

custom_derive! {
    #[derive(Debug, Copy, Clone, Eq, PartialEq, TryFrom(usize))]
    pub enum Size {
        Nine = 9,
        Thirteen = 13,
        Seventeen = 17,
        Nineteen = 19
    }
}

type StoneMap = BTreeMap<Coordinate, Stone>;

pub struct Game {
    id: u64,
    board: StoneMap,
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

// parse creates a new game from a simple human readable string representation.
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

// decode reads in the wire transfer format of the game.
pub fn decode(game_str: &str) -> Option<Game> {
    let segments : Vec<&str> = game_str.trim().split(";").collect();

    let size_value = match segments[0].parse::<usize>() {
        Ok(size_value) => size_value,
        _ => return None,
    };
    let size = match Size::try_from(size_value) {
        Ok(size) => size,
        _ => return None,
    };

    let mut board = BTreeMap::new();
    for (index, tile) in segments[1].chars().enumerate() {
        let x = index % size_value;
        let y = index / size_value;
        match tile {
            'b' => { board.insert((x as i8, y as i8), Stone::Black); },
            'w' => { board.insert((x as i8, y as i8), Stone::White); },
            _ => (),
        };
    };

    let turn = match segments[2] {
        "b" => Stone::Black,
        "w" => Stone::White,
        _ => return None,
    };

    Some(Game{
        id: 0,
        board: board,
        size: size,
        turn: turn})
}

// encode produces a tightly packed ASCII safe representation of a game that can be shipped over
// the wire safely.
pub fn encode(game: &Game) -> String {
    let mut output = "".to_string();

    let extent = game.size as i8;
    output.push_str(&*format!("{};", extent));

    for row in 0..extent {
        for column in 0..extent {
            output.push_str(match game.board.get(&(column, row)) {
                Some(&Stone::Black) => "b",
                Some(&Stone::White) => "w",
                None => ".",
            });
        }
    }
    output.push_str(";");
    match game.turn {
        Stone::Black => output.push_str("b"),
        Stone::White => output.push_str("w"),
    };
    output
}

impl Game {
    pub fn turn(&self) -> Stone {
        self.turn
    }

    // valid_coordinate determines if a coordinate is within the bounds of the game board.
    fn valid_coordinate(&self, (x, y): Coordinate) -> bool {
        let extent = self.size as i8;
        x >= 0 && x < extent && y >= 0 && y < extent
    }

    // adjacent_positions returns a list of valid adjacent coordinates.
    //
    // e.g:
    //  (0, 0) => [(1, 0), (0, 1]
    fn adjacent_positions(&self, (x, y): Coordinate) -> Vec<Coordinate> {
        [(x, y - 1), (x + 1, y), (x, y + 1), (x - 1, y)]
            .into_iter()
            .filter(|coordinate| self.valid_coordinate(**coordinate))
            .cloned()
            .collect()
    }

    // adjacent_liberties finds any adjacent tiles which do not contain a stone.
    // TODO: this also needs to check if the empty tile has a neighbouring tile which forms a chain
    // to another free tile.
    fn adjacent_liberties(&self, position: Coordinate) -> Vec<Coordinate> {
        self.adjacent_positions(position)
            .into_iter()
            .filter(|c| self.board.get(c).is_none())
            .collect()
    }

    // adjacent_stones returns a map of any adjacent stones.
    fn adjacent_stones(&self, position: Coordinate) -> StoneMap {
        self.adjacent_positions(position)
            .into_iter()
            .fold(StoneMap::new(), |mut accumulator, ref coordinate| {
                if let Some(piece) = self.board.get(coordinate) {
                    accumulator.entry(*coordinate).or_insert(*piece);
                }
                accumulator
            })
    }

    // allies returns adjacent tiles that contain the same stone.
    fn allies(&self, position: Coordinate, stone: Stone) -> StoneMap {
        self.adjacent_stones(position)
            .into_iter()
            .filter(|&(_, piece)| piece == stone)
            .collect()
    }

    // defenders returns adjacent tiles that contain the opponent's stones.
    fn defenders(&self, position: Coordinate, stone: Stone) -> StoneMap {
        self.adjacent_stones(position)
            .into_iter()
            .filter(|&(_, piece)| piece != stone)
            .collect()
    }

    fn is_safe(&self, position: Coordinate, stone: Stone) -> bool {
        self.adjacent_liberties(position).iter().count() > 0
    }

    fn is_suicide(&self, position: Coordinate, stone: Stone) -> bool {
        !self.is_safe(position, stone)
    }

    fn can_play(&self, position: Coordinate, stone: Stone) -> bool {
        self.turn == stone &&
            self.valid_coordinate(position) &&
            !self.has_stone(position) &&
            self.is_safe(position, stone)
    }

    fn advance_turn(&mut self) {
        self.turn = match self.turn {
            Stone::Black => Stone::White,
            Stone::White => Stone::Black,
        }
    }

    // chain returns a map of positions of adjacent stones of the same color which form a 'chain'.
    fn chain(&self, position: Coordinate, stone: Stone) -> StoneMap {
        let mut chain = StoneMap::new();
        chain.insert(position, stone);

        let mut searched_tiles = HashSet::<Coordinate>::new();
        let mut positions_to_search = Vec::<Coordinate>::new();
        positions_to_search.push(position);

        println!("looking for chain at {:?}", position);
        loop {
            println!("searched {:?}", searched_tiles);
            println!("positions_to_search {:?}", positions_to_search);

            let position = match positions_to_search.pop() {
                Some(position) => position,
                None => {
                    println!("chain {:?}", chain);
                    return chain;
                },
            };

            let searchable_tiles = self.allies(position, stone);
            for (position, _) in searchable_tiles.iter() {
                if !searched_tiles.contains(position) {
                    println!("haven't searched {:?}, adding", position);
                    positions_to_search.push(*position);
                    chain.insert(*position, stone);
                }
            }

            searched_tiles.insert(position);
        }
    }

    // remove_chain removes all pieces in a chain from the board.
    fn remove_chain(&mut self, chain: StoneMap) {
        for (position, stone) in chain.iter() {
            self.board.remove(position);
        }
    }

    // capture_stones removes any defenders from the board that no longer have any liberties.
    fn capture_stones(&mut self, position: Coordinate, stone: Stone) {
        let defenders = self.defenders(position, stone);
        println!("stone {:?} has defenders: {:?}", stone, defenders);

        for (position, stone) in defenders.iter() {
            let chain = self.chain(*position, *stone);
            println!("determining if chain {:?} has any liberties", chain);
            if chain.iter().all(|(position, stone)| !self.is_safe(*position, *stone)) {
                self.remove_chain(chain);
            }
        }
    }

    pub fn play_stone(&mut self, position: Coordinate, stone: Stone) -> bool {
        if self.can_play(position, stone) {
            self.board.insert(position, stone);
            self.capture_stones(position, stone);
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

impl fmt::Debug for Game {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let extent = self.size as i8;
        try!(formatter.write_str("\n\n"));
        for row in 0..extent {
            for column in 0..extent {
                try!(formatter.write_str(match self.board.get(&(column, row)) {
                    Some(&Stone::Black) => "b",
                    Some(&Stone::White) => "w",
                    None => ".",
                }))
            }
            try!(formatter.write_str("\n"));
        }
        formatter.write_str("\n")
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
    assert_eq!(false, game.play_stone((-1, 0), Stone::Black));
    assert_eq!(false, game.play_stone((-1, -1), Stone::Black));
    assert_eq!(false, game.play_stone((0, -1), Stone::Black));
    assert_eq!(false, game.play_stone((9, 0), Stone::Black));
    assert_eq!(false, game.play_stone((9, 9), Stone::Black));
    assert_eq!(false, game.play_stone((0, 9), Stone::Black));
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
..bbbbb..
.....b...
.........
.........
b.......b
.b.....b.", Stone::White).unwrap();

    // Top left corner
    assert_eq!(false, game.play_stone((0, 0), Stone::White));

    // Surrounded stone
    assert_eq!(false, game.play_stone((1, 1), Stone::White));

    // Bottom right corner
    assert_eq!(false, game.play_stone((8, 8), Stone::White));

    // Bottom left corner
    assert_eq!(false, game.play_stone((0, 8), Stone::White));

    // Top right corner
    assert_eq!(true, game.play_stone((8, 0), Stone::White));
    assert_eq!(false, game.play_stone((8, 1), Stone::White));
}

#[test]
fn test_play_stone_empty_neighbour() {
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

    assert_eq!(true, game.play_stone((2, 0), Stone::White));
}

#[test]
fn test_play_stone_capture_piece() {
    let mut game = parse("
.........
bwb......
.b.......
.........
.........
.........
.........
.........
.........", Stone::Black).unwrap();

    assert_eq!(true, game.play_stone((1, 0), Stone::Black));
    assert_eq!(false, game.has_stone((1, 1)));
    assert_eq!(Stone::Black, game.winner());
}

