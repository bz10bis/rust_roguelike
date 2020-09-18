use crate::object::Object;
use serde::{Deserialize, Serialize};
//game.rs

#[derive(Serialize, Deserialize)]
pub struct Game {
    pub map: Map,
    pub messages: Messages,
    pub inventory: Vec<Object>,
}

//map.rs
type Map = Vec<Vec<Tile>>;

#[derive(Clone, Copy, Debug)]
pub struct Rect {
   pub x1: i32,
   pub y1: i32,
   pub x2: i32,
   pub y2: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Rect {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h,
        }
    }

    pub fn center(&self) -> (i32, i32) {
        let center_x = (self.x1 + self.x2) / 2;
        let center_y = (self.y1 + self.y2) / 2;
        (center_x, center_y)
    }

    pub fn intersects_with(&self, other: &Rect) -> bool {
        (self.x1 <= other.x2)
            && (self.x2 >= other.x1)
            && (self.y1 <= other.y2)
            && (self.y2 >= other.y1)
    }
}


// ===================== TILE
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Tile {
    pub blocked: bool,
    pub block_sight: bool,
    pub explored: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile { blocked: false, block_sight: false, explored: false,}
    }

    pub fn wall() -> Self {
        Tile { blocked: true, block_sight: true, explored: false,}
    }
}

//messages.rs
//
use tcod::colors::*;

#[derive(Serialize, Deserialize)]
pub struct Messages {
    pub messages: Vec<(String, Color)>,

} 

impl Messages {
   pub fn new() -> Self {
       Self { messages: vec![] }
   }

   pub fn add<T: Into<String>>(&mut self, message: T, color: Color) {
       self.messages.push((message.into(), color));
   }

   pub fn iter(&self) -> impl DoubleEndedIterator<Item = &(String, Color)> {
       self.messages.iter()
   }
}

