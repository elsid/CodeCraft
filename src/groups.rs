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
    units: HashMap<i32, EntityType>,
    position: Vec2i,
    radius: i32,
    attack_range: i32,
    sight_range: i32,
    destroy_score: i32,
    damage: i32,
    health: i32,
}

impl Group {
    pub fn new(id: u32, need: HashMap<EntityType, usize>) -> Self {
        Self {
            id,
            state: GroupState::New,
            target: None,
            has: need.keys().cloned().map(|v| (v, 0)).collect(),
            need,
            units: HashMap::new(),
            position: Vec2i::zero(),
            radius: 0,
            attack_range: 0,
            sight_range: 0,
            destroy_score: 0,
            damage: 0,
            health: 0,
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
        let absent: Vec<i32> = self.units.keys().cloned().filter(|v| !world.contains_entity(*v)).collect();
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
            let mean_position = self.units.keys()
                .map(|v| world.get_entity(*v).position())
                .fold(Vec2i::zero(), |r, v| r + v) / self.units.len() as i32;
            self.position = self.units.keys()
                .map(|v| (world.get_entity(*v).position(), *v))
                .min_by_key(|(position, entity_id)| (position.distance(mean_position), *entity_id))
                .unwrap().0;
        }
        self.radius = self.units.iter()
            .map(|(v, _)| world.get_entity(*v).position().distance(self.position))
            .max()
            .unwrap_or(0);
        let mut sight_range = 0;
        let mut attack_range = 0;
        for (has, count) in self.has.iter() {
            if *count > 0 {
                attack_range = world.get_entity_properties(has).attack.as_ref()
                    .map(|v| v.attack_range.max(attack_range))
                    .unwrap_or(attack_range);
                sight_range = world.get_entity_properties(has).sight_range.max(sight_range);
            }
        }
        self.sight_range = sight_range;
        self.attack_range = attack_range;
        self.destroy_score = self.units.values()
            .map(|v| world.get_entity_properties(v).destroy_score)
            .sum();
        self.health = self.units.keys()
            .map(|v| world.get_entity(*v).health)
            .sum();
        self.damage = self.units.keys()
            .map(|v| {
                let entity = world.get_entity(*v);
                let properties = world.get_entity_properties(&entity.entity_type);
                properties.attack.as_ref().map(|v| v.damage).unwrap_or(0)
            })
            .sum();
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
