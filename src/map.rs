use std::iter::repeat;

#[cfg(feature = "enable_debug")]
use model::Color;
use model::PlayerView;

use crate::my_strategy::{Positionable, Vec2i};
#[cfg(feature = "enable_debug")]
use crate::my_strategy::{debug, Vec2f};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Tile {
    Unknown,
    Empty,
    Entity(i32),
    Outside,
}

pub struct Map {
    size: usize,
    tiles: Vec<Tile>,
    locked: Vec<bool>,
}

impl Map {
    pub fn new(player_view: &PlayerView) -> Self {
        let mut result = Self {
            size: player_view.map_size as usize,
            tiles: repeat(Tile::Unknown).take((player_view.map_size * player_view.map_size) as usize).collect(),
            locked: repeat(false).take((player_view.map_size * player_view.map_size) as usize).collect(),
        };
        result.update(player_view);
        result
    }

    pub fn update(&mut self, player_view: &PlayerView) {
        for i in 0..self.tiles.len() {
            self.tiles[i] = if player_view.fog_of_war {
                Tile::Unknown
            } else {
                Tile::Empty
            }
        }
        for entity in player_view.entities.iter() {
            let position = entity.position();
            let properties = &player_view.entity_properties[&entity.entity_type];
            for y in position.y()..position.y() + properties.size {
                for x in position.x()..position.x() + properties.size {
                    let index = self.get_tile_index(Vec2i::new(x, y));
                    self.tiles[index] = Tile::Entity(entity.id);
                }
            }
            if player_view.fog_of_war && entity.player_id == Some(player_view.my_id) {
                let position = entity.position();
                let entity_center = position + Vec2i::both(properties.size / 2);
                let sight_range = properties.sight_range + properties.size / 2 + (properties.size % 2 == 0) as i32;
                for y in (position.y() - properties.sight_range).max(0)..(position.y() + properties.size + properties.sight_range).min(self.size as i32) {
                    for x in (position.x() - properties.sight_range).max(0)..(position.x() + properties.size + properties.sight_range).min(self.size as i32) {
                        let tile_position = Vec2i::new(x, y);
                        let index = self.get_tile_index(tile_position);
                        if tile_position.distance(entity_center) <= sight_range
                            && matches!(self.tiles[index], Tile::Unknown) {
                            self.tiles[index] = Tile::Empty;
                        }
                    }
                }
            }
        }
    }

    pub fn contains(&self, position: Vec2i) -> bool {
        0 <= position.x() && position.x() < self.size as i32
            && 0 <= position.y() && position.y() < self.size as i32
    }

    pub fn get_tile(&self, position: Vec2i) -> Tile {
        self.tiles[self.get_tile_index(position)]
    }

    pub fn is_tile_locked(&self, position: Vec2i) -> bool {
        self.locked[self.get_tile_index(position)]
    }

    pub fn lock_tile(&mut self, position: Vec2i) {
        let index = self.get_tile_index(position);
        self.locked[index] = true;
    }

    pub fn lock_square(&mut self, position: Vec2i, size: i32) {
        find_inside_rect(
            position.highest(Vec2i::zero()),
            (position + Vec2i::both(size)).lowest(Vec2i::both(self.size as i32)),
            |tile_position| {
                self.lock_tile(tile_position);
                false
            },
        );
    }

    pub fn unlock_tile(&mut self, position: Vec2i) {
        let index = self.get_tile_index(position);
        self.locked[index] = false;
    }

    pub fn unlock_square(&mut self, position: Vec2i, size: i32) {
        find_inside_rect(
            position.highest(Vec2i::zero()),
            (position + Vec2i::both(size)).lowest(Vec2i::both(self.size as i32)),
            |tile_position| {
                self.unlock_tile(tile_position);
                false
            },
        );
    }

    fn get_tile_index(&self, position: Vec2i) -> usize {
        position_to_index(position, self.size)
    }

    pub fn find_inside_square<F: FnMut(Vec2i, Tile, bool) -> bool>(&self, position: Vec2i, size: i32, mut f: F) -> Option<Vec2i> {
        find_inside_rect(
            position,
            position + Vec2i::both(size),
            |tile_position| {
                let (tile, locked) = if self.contains(tile_position) {
                    (self.get_tile(tile_position), self.is_tile_locked(tile_position))
                } else {
                    (Tile::Outside, false)
                };
                f(tile_position, tile, locked)
            },
        )
    }

    pub fn find_inside_square_within_map<F: FnMut(Vec2i, Tile, bool) -> bool>(&self, position: Vec2i, size: i32, mut f: F) -> Option<Vec2i> {
        find_inside_rect(
            position.highest(Vec2i::zero()),
            (position + Vec2i::both(size)).lowest(Vec2i::both(self.size as i32)),
            |tile_position| {
                f(tile_position, self.get_tile(tile_position), self.is_tile_locked(tile_position))
            },
        )
    }

    pub fn find_neighbour<F: FnMut(Vec2i, Tile, bool) -> bool>(&self, position: Vec2i, size: i32, mut f: F) -> Option<Vec2i> {
        find_neighbour(
            position,
            size,
            |tile_position| {
                self.contains(tile_position) &&
                    f(tile_position, self.get_tile(tile_position), self.is_tile_locked(tile_position))
            },
        )
    }

    pub fn find_on_square_border<F: FnMut(Vec2i, Tile, bool) -> bool>(&self, position: Vec2i, size: i32, mut f: F) -> Option<Vec2i> {
        find_on_rect_border(
            position,
            position + Vec2i::both(size),
            |tile_position| {
                let (tile, locked) = if self.contains(tile_position) {
                    (self.get_tile(tile_position), self.is_tile_locked(tile_position))
                } else {
                    (Tile::Outside, false)
                };
                f(tile_position, tile, locked)
            },
        )
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, debug: &mut debug::Debug) {
        for i in 0..self.tiles.len() {
            let position = index_to_position(i, self.size);
            let color = match self.tiles[i] {
                Tile::Empty => Some(Color { a: 0.15, r: 0.0, g: 1.0, b: 1.0 }),
                Tile::Entity(..) => Some(Color { a: 0.25, r: 1.0, g: 0.0, b: 1.0 }),
                _ => None,
            };
            if let Some(color) = color {
                debug.add_world_square(
                    Vec2f::from(position) + Vec2f::new(0.25, 0.25),
                    0.5,
                    color,
                );
            }
            if self.locked[i] {
                debug.add_world_square(
                    Vec2f::from(position) + Vec2f::new(0.25, 0.25),
                    0.5,
                    Color { a: 0.5, r: 0.5, g: 0.0, b: 0.0 },
                );
            }
        }
    }
}

pub fn position_to_index(position: Vec2i, size: usize) -> usize {
    position.x() as usize + position.y() as usize * size
}

pub fn index_to_position(index: usize, size: usize) -> Vec2i {
    Vec2i::new((index % size as usize) as i32, (index / size as usize) as i32)
}

pub fn find_neighbour<F: FnMut(Vec2i) -> bool>(position: Vec2i, size: i32, mut f: F) -> Option<Vec2i> {
    for y in position.y()..position.y() + size {
        let tile_position = Vec2i::new(position.x() - 1, y);
        if f(tile_position) {
            return Some(tile_position);
        }
    }
    for x in position.x()..position.x() + size {
        let tile_position = Vec2i::new(x, position.y() + size);
        if f(tile_position) {
            return Some(tile_position);
        }
    }
    for y in position.y()..position.y() + size {
        let tile_position = Vec2i::new(position.x() + size, y);
        if f(tile_position) {
            return Some(tile_position);
        }
    }
    for x in position.x()..position.x() + size {
        let tile_position = Vec2i::new(x, position.y() - 1);
        if f(tile_position) {
            return Some(tile_position);
        }
    }
    None
}

pub fn find_inside_rect<F: FnMut(Vec2i) -> bool>(min: Vec2i, max: Vec2i, mut f: F) -> Option<Vec2i> {
    for y in min.y()..max.y() {
        for x in min.x()..max.x() {
            let tile_position = Vec2i::new(x, y);
            if f(tile_position) {
                return Some(tile_position);
            }
        }
    }
    None
}

pub fn find_on_rect_border<F: FnMut(Vec2i) -> bool>(min: Vec2i, max: Vec2i, mut f: F) -> Option<Vec2i> {
    for y in min.y()..max.y() - 1 {
        let tile_position = Vec2i::new(min.x(), y);
        if f(tile_position) {
            return Some(tile_position);
        }
    }
    for x in min.x()..max.x() - 1 {
        let tile_position = Vec2i::new(x, max.y() - 1);
        if f(tile_position) {
            return Some(tile_position);
        }
    }
    for y in min.y() + 1..max.y() {
        let tile_position = Vec2i::new(max.x() - 1, y);
        if f(tile_position) {
            return Some(tile_position);
        }
    }
    for x in min.x() + 1..max.x() {
        let tile_position = Vec2i::new(x, min.y());
        if f(tile_position) {
            return Some(tile_position);
        }
    }
    None
}
