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
    id: usize,
    state: GroupState,
    target: Option<Vec2i>,
    has: HashMap<EntityType, usize>,
    need: HashMap<EntityType, usize>,
    units: HashMap<i32, EntityType>,
}

impl Group {
    pub fn new(id: usize, need: HashMap<EntityType, usize>) -> Self {
        Self {
            id,
            state: GroupState::New,
            target: None,
            has: need.keys().cloned().map(|v| (v, 0)).collect(),
            need,
            units: HashMap::new(),
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn has(&self) -> &HashMap<EntityType, usize> {
        &self.has
    }

    pub fn update(&mut self, world: &World) {
        let absent: Vec<i32> = self.units.keys().cloned().filter(|v| !world.contains_entity(*v)).collect();
        for unit_id in absent.iter() {
            self.remove_unit(*unit_id);
        }
        for (entity_type, count) in self.need.iter_mut() {
            if !world.has_active_base_for(entity_type) {
                *count = self.has[&entity_type];
            }
        }
    }

    pub fn add_unit(&mut self, unit_id: i32, entity_type: EntityType) {
        *self.has.get_mut(&entity_type).unwrap() += 1;
        self.units.insert(unit_id, entity_type);
    }

    pub fn remove_unit(&mut self, unit_id: i32) {
        if let Some(entity_type) = self.units.remove(&unit_id) {
            *self.has.get_mut(&entity_type).unwrap() -= 1;
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

    pub fn get_center(&self, world: &World) -> Vec2i {
        self.units.keys()
            .fold(Vec2i::zero(), |r, unit_id| r + world.get_entity(*unit_id).position())
            / self.units.len() as i32
    }

    pub fn units(&self) -> impl Iterator<Item=&i32> {
        self.units.keys()
    }

    pub fn units_count(&self) -> usize {
        self.units.len()
    }

    pub fn need_count(&self) -> usize {
        self.need.values().fold(0, |r, v| r + *v)
    }

    pub fn get_bounds_min(&self, world: &World) -> Vec2i {
        self.units.keys()
            .fold(
                Vec2i::both(world.map_size()),
                |r, v| r.lowest(world.get_entity(*v).position()),
            )
    }

    pub fn get_bounds_max(&self, world: &World) -> Vec2i {
        self.units.keys()
            .fold(
                Vec2i::zero(),
                |r, v| {
                    let unit = world.get_entity(*v);
                    r.highest(unit.position() + Vec2i::both(world.get_entity_properties(&unit.entity_type).size))
                },
            )
    }
}
