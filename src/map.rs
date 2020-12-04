use std::iter::repeat;

#[cfg(feature = "enable_debug")]
use model::{
    Color,
    DebugCommand,
    DebugData,
    PrimitiveType,
};
use model::PlayerView;

#[cfg(feature = "enable_debug")]
use crate::DebugInterface;
use crate::my_strategy::{Positionable, Vec2i};
#[cfg(feature = "enable_debug")]
use crate::my_strategy::debug;
#[cfg(feature = "enable_debug")]
use crate::my_strategy::Vec2f;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Tile {
    Unknown,
    Empty,
    Entity(i32),
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
        visit_square(position, size, self.size as i32, |tile_position| {
            self.lock_tile(tile_position);
            true
        });
    }

    pub fn unlock_tile(&mut self, position: Vec2i) {
        let index = self.get_tile_index(position);
        self.locked[index] = false;
    }

    pub fn unlock_square(&mut self, position: Vec2i, size: i32) {
        visit_square(position, size, self.size as i32, |tile_position| {
            self.unlock_tile(tile_position);
            true
        });
    }

    fn get_tile_index(&self, position: Vec2i) -> usize {
        position.x() as usize + position.y() as usize * self.size
    }

    pub fn is_empty_square(&self, position: Vec2i, size: i32) -> bool {
        visit_square(position, size, self.size as i32, |tile_position| {
            matches!(self.get_tile(tile_position), Tile::Empty)
        })
    }

    pub fn is_free_square(&self, position: Vec2i, size: i32) -> bool {
        visit_square(position, size, self.size as i32, |tile_position| {
            !self.is_tile_locked(tile_position) && matches!(self.get_tile(tile_position), Tile::Empty)
        })
    }

    pub fn visit_neighbours<F: FnMut(Vec2i, Tile, bool) -> bool>(&self, position: Vec2i, size: i32, mut f: F) -> Option<Vec2i> {
        visit_neighbours(position, size, self.size as i32, move |tile_position| {
            f(tile_position, self.get_tile(tile_position), self.is_tile_locked(tile_position))
        })
    }

    pub fn visit_square_border<F: FnMut(Vec2i, Tile, bool) -> bool>(&self, position: Vec2i, size: i32, mut f: F) -> Option<Vec2i> {
        visit_square_border(position, size, self.size as i32, move |tile_position| {
            f(tile_position, self.get_tile(tile_position), self.is_tile_locked(tile_position))
        })
    }

    pub fn visit<F: FnMut(Vec2i, Tile, bool)>(&self, mut f: F) {
        for i in 0..self.tiles.len() {
            f(Vec2i::new((i % self.size) as i32, (i / self.size) as i32), self.tiles[i], self.locked[i]);
        }
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, debug: &mut DebugInterface) {
        let mut vertices = Vec::new();
        self.visit(|position, tile, locked| {
            let color = match tile {
                Tile::Empty => Some(Color { a: 0.15, r: 0.0, g: 1.0, b: 1.0 }),
                Tile::Entity(..) => Some(Color { a: 0.25, r: 1.0, g: 0.0, b: 1.0 }),
                _ => None,
            };
            if let Some(color) = color {
                debug::add_world_square(Vec2f::from(position), 1.0, color, &mut vertices);
            }
            if locked {
                debug::add_world_square(
                    Vec2f::from(position),
                    1.0,
                    Color { a: 0.5, r: 0.5, g: 0.0, b: 0.0 },
                    &mut vertices,
                );
            }
        });
        debug.send(DebugCommand::Add {
            data: DebugData::Primitives {
                vertices,
                primitive_type: PrimitiveType::Triangles,
            }
        });
    }
}

pub fn visit_neighbours<F: FnMut(Vec2i) -> bool>(position: Vec2i, size: i32, max_size: i32, mut f: F) -> Option<Vec2i> {
    if position.x() > 0 {
        for y in position.y()..position.y() + size {
            let tile_position = Vec2i::new(position.x() - 1, y);
            if !f(tile_position) {
                return Some(tile_position);
            }
        }
    }
    if position.y() + size < max_size {
        for x in position.x()..position.x() + size {
            let tile_position = Vec2i::new(x, position.y() + size);
            if !f(tile_position) {
                return Some(tile_position);
            }
        }
    }
    if position.x() + size < max_size {
        for y in position.y()..position.y() + size {
            let tile_position = Vec2i::new(position.x() + size, y);
            if !f(tile_position) {
                return Some(tile_position);
            }
        }
    }
    if position.y() > 0 {
        for x in position.x()..position.x() + size {
            let tile_position = Vec2i::new(x, position.y() - 1);
            if !f(tile_position) {
                return Some(tile_position);
            }
        }
    }
    None
}

pub fn visit_square<F: FnMut(Vec2i) -> bool>(position: Vec2i, size: i32, max_size: i32, mut f: F) -> bool {
    for y in position.y().max(0)..(position.y() + size).min(max_size) {
        for x in position.x().max(0)..(position.x() + size).min(max_size) {
            if !f(Vec2i::new(x, y)) {
                return false;
            }
        }
    }
    true
}

pub fn visit_square_border<F: FnMut(Vec2i) -> bool>(position: Vec2i, size: i32, max_size: i32, mut f: F) -> Option<Vec2i> {
    if position.x() > 0 {
        for y in position.y().max(0)..(position.y() + size).min(max_size) {
            let tile_position = Vec2i::new(position.x(), y);
            if !f(tile_position) {
                return Some(tile_position);
            }
        }
    }
    if position.y() + size < max_size as i32 {
        for x in position.x().max(0)..(position.x() + size).min(max_size) {
            let tile_position = Vec2i::new(x, position.y() + size);
            if !f(tile_position) {
                return Some(tile_position);
            }
        }
    }
    if position.x() + size < max_size as i32 {
        for y in position.y().max(0)..(position.y() + size).min(max_size) {
            let tile_position = Vec2i::new(position.x() + size, y);
            if !f(tile_position) {
                return Some(tile_position);
            }
        }
    }
    if position.y() > 0 {
        for x in position.x().max(0)..(position.x() + size).min(max_size) {
            let tile_position = Vec2i::new(x, position.y());
            if !f(tile_position) {
                return Some(tile_position);
            }
        }
    }
    None
}
