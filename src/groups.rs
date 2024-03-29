use std::collections::HashMap;

use model::EntityType;

use crate::my_strategy::{Positionable, Vec2i, World};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum GroupState {
    New,
    Ready,
}

#[derive(Debug)]
pub struct Group {
    id: u32,
    state: GroupState,
    target: Option<Vec2i>,
    has: HashMap<EntityType, usize>,
    need: HashMap<EntityType, usize>,
    units: Vec<(i32, EntityType)>,
    position: Vec2i,
}

impl Group {
    pub fn new(id: u32, need: HashMap<EntityType, usize>) -> Self {
        Self {
            id,
            state: GroupState::New,
            target: None,
            has: need.keys().cloned().map(|v| (v, 0)).collect(),
            need,
            units: Vec::new(),
            position: Vec2i::zero(),
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn has(&self) -> &HashMap<EntityType, usize> {
        &self.has
    }

    pub fn position(&self) -> Vec2i {
        self.position
    }

    pub fn update(&mut self, world: &World) {
        let absent: Vec<i32> = self.units.iter()
            .filter(|(entity_id, _)| !world.contains_entity(*entity_id))
            .map(|(entity_id, _)| *entity_id)
            .collect();
        for unit_id in absent.iter() {
            self.remove_unit(*unit_id);
        }
        for (entity_type, count) in self.need.iter_mut() {
            if !world.has_active_base_for(entity_type) {
                *count = self.has[&entity_type];
            }
        }
        if self.units.is_empty() {
            self.position = Vec2i::zero();
        } else {
            let mean_position = self.units.iter()
                .map(|(entity_id, _)| world.get_entity(*entity_id).position())
                .fold(Vec2i::zero(), |r, v| r + v) / self.units.len() as i32;
            self.position = self.units.iter()
                .map(|(entity_id, _)| world.get_entity(*entity_id).position())
                .min_by_key(|position| position.distance(mean_position))
                .unwrap();
        }
    }

    pub fn add_unit(&mut self, unit_id: i32, entity_type: EntityType) {
        *self.has.get_mut(&entity_type).unwrap() += 1;
        self.units.push((unit_id, entity_type));
    }

    pub fn remove_unit(&mut self, unit_id: i32) {
        if let Some((_, entity_type)) = self.units.iter().find(|(entity_id, _)| *entity_id == unit_id).cloned() {
            *self.has.get_mut(&entity_type).unwrap() -= 1;
            self.units.retain(|(entity_id, _)| *entity_id != unit_id);
        }
    }

    pub fn clear(&mut self) {
        self.units.clear();
        for count in self.has.values_mut() {
            *count = 0;
        }
    }

    pub fn set_target(&mut self, value: Option<Vec2i>) {
        self.target = value;
    }

    pub fn target(&self) -> Option<Vec2i> {
        self.target
    }

    pub fn is_full(&self) -> bool {
        self.need.iter().all(|(k, v)| *v <= self.has[k])
    }

    pub fn need_more_of(&self, entity_type: &EntityType) -> bool {
        self.need.get(entity_type).map(|v| *v > self.has[entity_type]).unwrap_or(false)
    }

    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    pub fn set_state(&mut self, value: GroupState) {
        self.state = value;
    }

    pub fn state(&self) -> GroupState {
        self.state
    }

    pub fn units(&self) -> &Vec<(i32, EntityType)> {
        &self.units
    }

    pub fn units_count(&self) -> usize {
        self.units.len()
    }

    pub fn need_count(&self) -> usize {
        self.need.values().fold(0, |r, v| r + *v)
    }

    #[cfg(feature = "enable_debug")]
    pub fn get_bounds_min(&self, world: &World) -> Vec2i {
        self.units.iter()
            .map(|(entity_id, _)| *entity_id)
            .fold(
                Vec2i::both(world.map_size()),
                |r, entity_id| r.lowest(world.get_entity(entity_id).position()),
            )
    }

    #[cfg(feature = "enable_debug")]
    pub fn get_bounds_max(&self, world: &World) -> Vec2i {
        self.units.iter()
            .map(|(entity_id, _)| *entity_id)
            .fold(
                Vec2i::zero(),
                |r, entity_id| {
                    let unit = world.get_entity(entity_id);
                    r.highest(unit.position() + Vec2i::both(world.get_entity_properties(&unit.entity_type).size))
                },
            )
    }
}
