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
    sight_range: i32,
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
            sight_range: 0,
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

    pub fn sight_range(&self) -> i32 {
        self.sight_range
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
            self.position =  world.get_entity(self.units[0].0).position();
        }
        let mut sight_range = 0;
        for (has, count) in self.has.iter() {
            if *count > 0 {
                sight_range = world.get_entity_properties(has).sight_range.max(sight_range);
            }
        }
        self.sight_range = sight_range;
    }

    pub fn add_unit(&mut self, unit_id: i32, entity_type: EntityType) {
        *self.has.get_mut(&entity_type).unwrap() += 1;
        self.units.push((unit_id, entity_type));
    }

    pub fn remove_unit(&mut self, unit_id: i32) {
        if let Some((_, entity_type)) = self.units.iter().find(|(entity_id, _)| *entity_id == unit_id) {
            *self.has.get_mut(&entity_type).unwrap() -= 1;
        }
        self.units.retain(|(entity_id, _)| *entity_id != unit_id);
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
            .map(|(unit_id, _)| unit_id)
            .fold(
                Vec2i::both(world.map_size()),
                |r, v| r.lowest(world.get_entity(*v).position()),
            )
    }

    #[cfg(feature = "enable_debug")]
    pub fn get_bounds_max(&self, world: &World) -> Vec2i {
        self.units.iter()
            .map(|(unit_id, _)| unit_id)
            .fold(
                Vec2i::zero(),
                |r, v| {
                    let unit = world.get_entity(*v);
                    r.highest(unit.position() + Vec2i::both(world.get_entity_properties(&unit.entity_type).size))
                },
            )
    }
}
