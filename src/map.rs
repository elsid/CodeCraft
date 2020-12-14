use std::iter::repeat;

use model::{Entity, EntityProperties};
#[cfg(feature = "enable_debug")]
use model::Color;

use crate::my_strategy::{Positionable, Rect, Vec2i};
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
    cached: Vec<bool>,
}

impl Map {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            tiles: repeat(Tile::Unknown).take(size * size).collect(),
            locked: repeat(false).take(size * size).collect(),
            cached: repeat(false).take(size * size).collect(),
        }
    }

    pub fn update_with_actual(&mut self, player_id: i32, fog_of_war: bool, entities: &Vec<Entity>, entity_properties: &Vec<EntityProperties>) {
        for i in 0..self.tiles.len() {
            self.tiles[i] = if fog_of_war {
                Tile::Unknown
            } else {
                Tile::Empty
            };
        }
        if fog_of_war {
            for value in self.cached.iter_mut() {
                *value = true;
            }
        }
        for entity in entities.iter() {
            let size = entity_properties[entity.entity_type.clone() as usize].size;
            let map_size = self.size;
            visit_square(entity.position(), size, |position| {
                let index = position_to_index(position, map_size);
                self.tiles[index] = Tile::Entity(entity.id);
                self.cached[index] = false;
            });
        }
        if fog_of_war {
            for entity in entities.iter() {
                if entity.player_id == Some(player_id) {
                    let position = entity.position();
                    let properties = &entity_properties[entity.entity_type.clone() as usize];
                    let bounds = Rect::new(Vec2i::zero(), Vec2i::both(self.size as i32));
                    visit_range(position, properties.size, properties.sight_range, &bounds, |tile_position| {
                        let index = self.get_tile_index(tile_position);
                        match self.tiles[index] {
                            Tile::Unknown => {
                                self.tiles[index] = Tile::Empty;
                                self.cached[index] = false;
                            }
                            _ => (),
                        }
                    });
                }
            }
        }
    }

    pub fn update_with_cached(&mut self, entities: &Vec<Entity>, entity_properties: &Vec<EntityProperties>) {
        for entity in entities.iter() {
            let size = entity_properties[entity.entity_type.clone() as usize].size;
            let busy = find_inside_square(entity.position(), size, |position| {
                match self.tiles[self.get_tile_index(position)] {
                    Tile::Entity(entity_id) => entity_id != entity.id,
                    Tile::Empty => true,
                    _ => false,
                }
            }).is_some();
            if !busy {
                let map_size = self.size;
                visit_square(entity.position(), size, |position| {
                    self.tiles[position_to_index(position, map_size)] = Tile::Entity(entity.id);
                });
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

    pub fn is_tile_cached(&self, position: Vec2i) -> bool {
        self.cached[self.get_tile_index(position)]
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

    pub fn count_unknown_tiles(&self) -> usize {
        self.tiles.iter().filter(|v| matches!(**v, Tile::Unknown)).count()
    }

    fn get_tile_index(&self, position: Vec2i) -> usize {
        position_to_index(position, self.size)
    }

    pub fn visit_square<F: FnMut(Vec2i, Tile, bool)>(&self, position: Vec2i, size: i32, mut f: F) {
        visit_square(position, size, |tile_position| {
            f(tile_position, self.get_tile(tile_position), self.is_tile_locked(tile_position))
        });
    }

    pub fn visit_range<F: FnMut(Vec2i, Tile, bool)>(&self, position: Vec2i, size: i32, range: i32, mut f: F) {
        let bounds = Rect::new(Vec2i::zero(), Vec2i::both(self.size as i32));
        visit_range(position, size, range, &bounds, |tile_position| {
            f(tile_position, self.get_tile(tile_position), self.is_tile_locked(tile_position))
        });
    }

    pub fn find_in_range<F: FnMut(Vec2i, Tile, bool) -> bool>(&self, position: Vec2i, size: i32, range: i32, mut f: F) -> Option<Vec2i> {
        let bounds = Rect::new(Vec2i::zero(), Vec2i::both(self.size as i32));
        find_in_range(position, size, range, &bounds, |tile_position| {
            f(tile_position, self.get_tile(tile_position), self.is_tile_locked(tile_position))
        })
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

pub fn visit_neighbour<F: FnMut(Vec2i)>(position: Vec2i, size: i32, mut f: F) {
    find_neighbour(position, size, |tile_position| {
        f(tile_position);
        false
    });
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

pub fn visit_range<F: FnMut(Vec2i)>(position: Vec2i, size: i32, range: i32, bounds: &Rect, mut f: F) {
    let bottom = position.y() + size;
    for y in (position.y() - range).max(bounds.min().y())..(bottom + range).min(bounds.max().y()) {
        let shift = if y < position.y() {
            range - (position.y() - y)
        } else if y >= bottom {
            range - (y - (bottom - 1))
        } else {
            range
        };
        for x in (position.x() - shift).max(bounds.min().x())..(position.x() + size + shift).min(bounds.max().x()) {
            f(Vec2i::new(x, y))
        }
    }
}

pub fn find_in_range<F: FnMut(Vec2i) -> bool>(position: Vec2i, size: i32, range: i32, bounds: &Rect, mut f: F) -> Option<Vec2i> {
    for y in (position.y() - range).max(bounds.min().y())..(position.y() + size + range).min(bounds.max().y()) {
        let shift = if y < position.y() {
            range - (position.y() - y)
        } else if y >= position.y() + size {
            range - (y - (position.y() + size - 1))
        } else {
            range
        };
        for x in (position.x() - shift).max(bounds.min().x())..(position.x() + size + shift).min(bounds.max().x()) {
            let tile_position = Vec2i::new(x, y);
            if f(tile_position) {
                return Some(tile_position);
            }
        }
    }
    None
}

pub fn visit_square_with_bounds<F: FnMut(Vec2i)>(position: Vec2i, size: i32, bounds: &Rect, mut f: F) {
    for y in position.y().max(bounds.min().y())..(position.y() + size).min(bounds.max().y()) {
        for x in position.x().max(bounds.min().x())..(position.x() + size).min(bounds.max().x()) {
            f(Vec2i::new(x, y))
        }
    }
}

pub fn visit_square<F: FnMut(Vec2i)>(position: Vec2i, size: i32, mut f: F) {
    for y in position.y()..position.y() + size {
        for x in position.x()..position.x() + size {
            f(Vec2i::new(x, y))
        }
    }
}

pub fn find_inside_square<F: FnMut(Vec2i) -> bool>(position: Vec2i, size: i32, mut f: F) -> Option<Vec2i> {
    for y in position.y()..position.y() + size {
        for x in position.x()..position.x() + size {
            let tile_position = Vec2i::new(x, y);
            if f(tile_position) {
                return Some(tile_position);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::my_strategy::{Rect, Vec2i, visit_range};

    #[test]
    fn visit_range_1_1() {
        let mut result = Vec::new();
        visit_range(Vec2i::new(5, 5), 1, 1, &Rect::new(Vec2i::zero(), Vec2i::both(80)), |position| {
            result.push(position);
        });
        assert_eq!(result, vec![
            Vec2i::new(5, 4),
            Vec2i::new(4, 5),
            Vec2i::new(5, 5),
            Vec2i::new(6, 5),
            Vec2i::new(5, 6),
        ]);
    }

    #[test]
    fn visit_range_2_1() {
        let mut result = Vec::new();
        visit_range(Vec2i::new(5, 5), 2, 1, &Rect::new(Vec2i::zero(), Vec2i::both(80)), |position| {
            result.push(position);
        });
        assert_eq!(result, vec![
            Vec2i::new(5, 4), Vec2i::new(6, 4),
            Vec2i::new(4, 5), Vec2i::new(5, 5), Vec2i::new(6, 5), Vec2i::new(7, 5),
            Vec2i::new(4, 6), Vec2i::new(5, 6), Vec2i::new(6, 6), Vec2i::new(7, 6),
            Vec2i::new(5, 7), Vec2i::new(6, 7),
        ]);
    }

    #[test]
    fn visit_range_2_2() {
        let mut result = Vec::new();
        visit_range(Vec2i::new(5, 5), 2, 2, &Rect::new(Vec2i::zero(), Vec2i::both(80)), |position| {
            result.push(position);
        });
        assert_eq!(result, vec![
            Vec2i::new(5, 3), Vec2i::new(6, 3),
            Vec2i::new(4, 4), Vec2i::new(5, 4), Vec2i::new(6, 4), Vec2i::new(7, 4),
            Vec2i::new(3, 5), Vec2i::new(4, 5), Vec2i::new(5, 5), Vec2i::new(6, 5), Vec2i::new(7, 5), Vec2i::new(8, 5),
            Vec2i::new(3, 6), Vec2i::new(4, 6), Vec2i::new(5, 6), Vec2i::new(6, 6), Vec2i::new(7, 6), Vec2i::new(8, 6),
            Vec2i::new(4, 7), Vec2i::new(5, 7), Vec2i::new(6, 7), Vec2i::new(7, 7),
            Vec2i::new(5, 8), Vec2i::new(6, 8),
        ]);
    }
}
