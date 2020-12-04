use std::cell::RefCell;
use std::collections::HashMap;

use model::{
    Entity,
    EntityProperties,
    EntityType,
    Player,
    PlayerView,
};

use crate::DebugInterface;
use crate::my_strategy::{
    is_entity_base,
    is_entity_unit,
    Map,
    Positionable,
    Tile,
    Vec2i,
};

pub struct World {
    my_id: i32,
    map_size: i32,
    fog_of_war: bool,
    entity_properties: HashMap<EntityType, EntityProperties>,
    max_tick_count: i32,
    max_pathfind_nodes: i32,
    current_tick: i32,
    players: Vec<Player>,
    entities: Vec<Entity>,
    entities_by_id: HashMap<i32, usize>,
    my_entities_count: HashMap<EntityType, usize>,
    map: RefCell<Map>,
    population_use: i32,
    population_provide: i32,
    start_position: Vec2i,
    base_size: RefCell<Option<i32>>,
    requested_resource: RefCell<i32>,
    allocated_resource: RefCell<i32>,
    allocated_population: RefCell<i32>,
    protected_radius: RefCell<Option<i32>>,
}

impl World {
    pub fn new(player_view: &PlayerView) -> Self {
        Self {
            my_id: player_view.my_id,
            map_size: player_view.map_size,
            fog_of_war: player_view.fog_of_war,
            entity_properties: player_view.entity_properties.clone(),
            max_tick_count: player_view.max_tick_count,
            max_pathfind_nodes: player_view.max_pathfind_nodes,
            current_tick: player_view.current_tick,
            players: Vec::new(),
            entities: Vec::new(),
            entities_by_id: HashMap::new(),
            my_entities_count: player_view.entity_properties.keys()
                .map(|v| (v.clone(), 0))
                .collect(),
            map: RefCell::new(Map::new(player_view)),
            population_use: 0,
            population_provide: 0,
            start_position: player_view.entities.iter()
                .find(|v| v.player_id == Some(player_view.my_id) && matches!(v.entity_type, EntityType::BuilderUnit)).unwrap()
                .position(),
            base_size: RefCell::new(None),
            requested_resource: RefCell::new(0),
            allocated_resource: RefCell::new(0),
            allocated_population: RefCell::new(0),
            protected_radius: RefCell::new(None),
        }
    }

    pub fn update(&mut self, player_view: &PlayerView) {
        self.current_tick = player_view.current_tick;
        self.players = player_view.players.clone();
        self.players.sort_by_key(|v| v.id);
        self.entities = player_view.entities.clone();
        self.entities.sort_by_key(|v| v.id);
        self.entities_by_id = self.entities.iter().enumerate().map(|(n, v)| (v.id, n)).collect();
        for count in self.my_entities_count.values_mut() {
            *count = 0;
        }
        let entities_count = &mut self.my_entities_count;
        for entity in self.entities.iter() {
            if entity.player_id == Some(self.my_id) {
                *entities_count.get_mut(&entity.entity_type).unwrap() += 1;
            }
        }
        self.map.borrow_mut().update(player_view);
        self.population_use = self.my_entities().map(|v| self.get_entity_properties(&v.entity_type).population_use).sum();
        self.population_provide = self.my_entities().map(|v| self.get_entity_properties(&v.entity_type).population_provide).sum();
        *self.base_size.borrow_mut() = None;
        *self.allocated_resource.borrow_mut() = 0;
        *self.allocated_population.borrow_mut() = 0;
        *self.protected_radius.borrow_mut() = None;
    }

    pub fn my_id(&self) -> i32 {
        self.my_id
    }

    pub fn map_size(&self) -> i32 {
        self.map_size
    }

    pub fn current_tick(&self) -> i32 {
        self.current_tick
    }

    pub fn get_entity_properties(&self, entity_type: &EntityType) -> &EntityProperties {
        &self.entity_properties[entity_type]
    }

    pub fn players(&self) -> &Vec<Player> {
        &self.players
    }

    pub fn contains(&self, position: Vec2i) -> bool {
        self.map.borrow().contains(position)
    }

    pub fn get_tile(&self, position: Vec2i) -> Tile {
        self.map.borrow().get_tile(position)
    }

    pub fn is_free_square(&self, position: Vec2i, size: i32) -> bool {
        self.map.borrow().is_free_square(position, size)
    }

    pub fn population_use(&self) -> i32 {
        self.population_use
    }

    pub fn population_provide(&self) -> i32 {
        self.population_provide
    }

    pub fn start_position(&self) -> Vec2i {
        self.start_position
    }

    pub fn get_entity(&self, entity_id: i32) -> &Entity {
        &self.entities[self.entities_by_id[&entity_id]]
    }

    pub fn find_entity(&self, entity_id: i32) -> Option<&Entity> {
        self.entities_by_id.get(&entity_id).map(|v| &self.entities[*v])
    }

    pub fn contains_entity(&self, entity_id: i32) -> bool {
        self.entities_by_id.contains_key(&entity_id)
    }

    pub fn resources(&self) -> impl Iterator<Item=&Entity> {
        self.entities.iter()
            .filter(|v| v.entity_type == EntityType::Resource)
    }

    pub fn get_player(&self, player_id: i32) -> &Player {
        self.players.iter().find(|v| v.id == player_id).unwrap()
    }

    pub fn my_player(&self) -> &Player {
        self.players.iter().find(|v| v.id == self.my_id).unwrap()
    }

    pub fn my_entities(&self) -> impl Iterator<Item=&Entity> {
        let my_id = self.my_id;
        self.entities.iter()
            .filter(move |v| v.player_id == Some(my_id))
    }

    pub fn opponent_entities(&self) -> impl Iterator<Item=&Entity> {
        let my_id = self.my_id;
        self.entities.iter()
            .filter(move |v| v.player_id.map(|v| v != my_id).unwrap_or(false))
    }

    pub fn my_turrets(&self) -> impl Iterator<Item=&Entity> {
        self.my_entities()
            .filter(|v| matches!(v.entity_type, EntityType::Turret))
    }

    pub fn my_buildings(&self) -> impl Iterator<Item=&Entity> {
        let entity_properties = &self.entity_properties;
        self.my_entities()
            .filter(move |v| !entity_properties[&v.entity_type].can_move)
    }

    pub fn my_bases(&self) -> impl Iterator<Item=&Entity> {
        self.my_entities()
            .filter(|v| is_entity_base(v))
    }

    pub fn my_builder_bases(&self) -> impl Iterator<Item=&Entity> {
        self.my_entities()
            .filter(|v| matches!(v.entity_type, EntityType::BuilderBase))
    }

    pub fn my_melee_bases(&self) -> impl Iterator<Item=&Entity> {
        self.my_entities()
            .filter(|v| matches!(v.entity_type, EntityType::MeleeBase))
    }

    pub fn my_ranged_bases(&self) -> impl Iterator<Item=&Entity> {
        self.my_entities()
            .filter(|v| matches!(v.entity_type, EntityType::RangedBase))
    }

    pub fn my_units(&self) -> impl Iterator<Item=&Entity> {
        self.my_entities()
            .filter(|v| is_entity_unit(v))
    }

    pub fn my_builder_units(&self) -> impl Iterator<Item=&Entity> {
        self.my_entities()
            .filter(|v| matches!(v.entity_type, EntityType::BuilderUnit))
    }

    pub fn my_ranged_units(&self) -> impl Iterator<Item=&Entity> {
        self.my_entities()
            .filter(|v| matches!(v.entity_type, EntityType::RangedUnit))
    }

    pub fn my_melee_units(&self) -> impl Iterator<Item=&Entity> {
        self.my_entities()
            .filter(|v| matches!(v.entity_type, EntityType::MeleeUnit))
    }

    pub fn is_empty_square(&self, position: Vec2i, size: i32) -> bool {
        self.map.borrow().is_empty_square(position, size)
    }

    pub fn find_free_tile_nearby(&self, position: Vec2i, size: i32) -> Option<Vec2i> {
        self.map.borrow()
            .visit_neighbours(position, size, |_, tile, locked| {
                locked || !matches!(tile, Tile::Empty)
            })
    }

    pub fn find_nearest_free_tile_nearby_for_unit(&self, position: Vec2i, size: i32, unit_id: i32) -> Option<Vec2i> {
        let unit_position = self.get_entity(unit_id).position();
        let mut result = None;
        self.map.borrow()
            .visit_neighbours(position, size, |tile_position, tile, locked| {
                if !locked && (matches!(tile, Tile::Empty) || tile == Tile::Entity(unit_id))
                    && result.map(|v| unit_position.distance(v) > unit_position.distance(tile_position)).unwrap_or(true) {
                    result = Some(tile_position);
                }
                true
            });
        result
    }

    pub fn find_free_space(&self, position: Vec2i, size: i32, min_radius: i32, max_radius: i32) -> Option<Vec2i> {
        let map = self.map.borrow();
        if map.is_free_square(position, size) {
            return Some(position);
        }
        for radius in min_radius.max(1)..max_radius.min(self.map_size) {
            let result = map.visit_square_border(
                position - Vec2i::both(radius),
                2 * radius + 1,
                |v, _, _| !map.is_free_square(v, size),
            );
            if result.is_some() {
                return result;
            }
        }
        None
    }

    pub fn get_base_size(&self) -> i32 {
        if let Some(base_size) = self.base_size.borrow().as_ref() {
            return *base_size;
        }
        let (max, min) = self.my_bases()
            .filter(|v| !matches!(v.entity_type, EntityType::Turret))
            .fold(
                (Vec2i::zero(), Vec2i::both(self.map_size)),
                |(max, min), v| {
                    (max.max(v.position() + Vec2i::both(self.get_entity_properties(&v.entity_type).size)), min.min(v.position()))
                },
            );
        let result = (max.x() - min.x()).max(max.y() - min.y());
        *self.base_size.borrow_mut() = Some(result);
        result
    }

    pub fn my_resource(&self) -> i32 {
        self.my_player().resource
            - *self.requested_resource.borrow()
            - *self.allocated_resource.borrow()
    }

    pub fn allocated_resource(&self) -> i32 {
        *self.allocated_resource.borrow()
    }

    pub fn requested_resource(&self) -> i32 {
        *self.requested_resource.borrow()
    }

    pub fn try_allocate_resource(&self, amount: i32) -> bool {
        if self.my_resource() < amount {
            return false;
        }
        *self.allocated_resource.borrow_mut() += amount;
        true
    }

    pub fn try_request_resources(&self, amount: i32) -> bool {
        if self.my_resource() <= 0 {
            return false;
        }
        *self.requested_resource.borrow_mut() += amount;
        true
    }

    pub fn release_requested_resource(&self, amount: i32) {
        *self.requested_resource.borrow_mut() -= amount;
    }

    pub fn my_population(&self) -> i32 {
        self.population_use - *self.allocated_population.borrow()
    }

    pub fn allocated_population(&self) -> i32 {
        *self.allocated_population.borrow()
    }

    pub fn try_allocate_population(&self, amount: i32) -> bool {
        if self.my_population() < amount {
            return false;
        }
        *self.allocated_population.borrow_mut() += amount;
        true
    }

    pub fn try_allocated_resource_and_population(&self, resource: i32, population: i32) -> bool {
        if self.my_resource() < resource || self.my_population() < population {
            return false;
        }
        *self.allocated_resource.borrow_mut() += resource;
        *self.allocated_population.borrow_mut() += population;
        true
    }

    pub fn lock_square(&self, position: Vec2i, size: i32) {
        self.map.borrow_mut().lock_square(position, size);
    }

    pub fn unlock_square(&self, position: Vec2i, size: i32) {
        self.map.borrow_mut().unlock_square(position, size);
    }

    pub fn debug_update(&self, debug: &mut DebugInterface) {
        self.map.borrow().debug_update(debug);
    }

    pub fn get_my_builder_units_count(&self) -> usize {
        self.my_entities_count[&EntityType::BuilderUnit]
    }

    pub fn get_my_melee_units_count(&self) -> usize {
        self.my_entities_count[&EntityType::MeleeUnit]
    }

    pub fn get_my_ranged_units_count(&self) -> usize {
        self.my_entities_count[&EntityType::RangedUnit]
    }

    pub fn get_my_builder_bases_count(&self) -> usize {
        self.my_entities_count[&EntityType::BuilderBase]
    }

    pub fn get_my_melee_bases_count(&self) -> usize {
        self.my_entities_count[&EntityType::MeleeBase]
    }

    pub fn get_my_ranged_bases_count(&self) -> usize {
        self.my_entities_count[&EntityType::RangedBase]
    }

    pub fn get_my_turrets_count(&self) -> usize {
        self.my_entities_count[&EntityType::Turret]
    }

    pub fn get_my_units_count(&self) -> usize {
        self.my_entities_count.iter()
            .filter(|(k, _)| self.entity_properties[k].can_move)
            .map(|(_, v)| *v)
            .sum()
    }

    pub fn get_my_buildings_count(&self) -> usize {
        self.my_entities_count.iter()
            .filter(|(k, _)| !self.entity_properties[k].can_move)
            .map(|(_, v)| *v)
            .sum()
    }

    pub fn get_entity_cost(&self, entity_type: &EntityType) -> i32 {
        self.entity_properties[entity_type].initial_cost + self.my_entities_count[entity_type] as i32
    }

    pub fn is_attacked_by_opponents(&self, position: Vec2i) -> bool {
        self.opponent_entities()
            .filter_map(|entity| {
                self.entity_properties[&entity.entity_type].attack.as_ref()
                    .map(|v| (entity.position(), v.attack_range.max(3)))
            })
            .any(|(entity_position, attack_range)| {
                entity_position.distance(position) <= attack_range
            })
    }

    pub fn distance_to_nearest_opponent(&self, position: Vec2i) -> Option<i32> {
        self.opponent_entities()
            .filter(|entity| self.entity_properties[&entity.entity_type].attack.is_some())
            .map(|entity| entity.position().distance(position))
            .min()
    }

    pub fn get_protected_radius(&self) -> i32 {
        if let Some(v) = *self.protected_radius.borrow() {
            return v;
        }
        let result = self.my_entities()
            .filter(|v| is_protected_entity_type(&v.entity_type))
            .map(|v| v.position().distance(self.start_position) + self.entity_properties[&v.entity_type].sight_range)
            .max();
        *self.protected_radius.borrow_mut() = result;
        result.unwrap_or(0)
    }

    pub fn is_inside_protected_perimeter(&self, position: Vec2i) -> bool {
        position.distance(self.start_position) <= self.get_protected_radius()
    }

    pub fn has_active_base_for(&self, entity_type: &EntityType) -> bool {
        for base in self.my_bases() {
            if base.active {
                if let Some(build) = self.get_entity_properties(&base.entity_type).build.as_ref() {
                    if build.options[0] == *entity_type {
                        return true;
                    }
                }
            }
        }
        false
    }
}

pub fn is_protected_entity_type(entity_type: &EntityType) -> bool {
    match entity_type {
        EntityType::Turret => true,
        EntityType::House => true,
        EntityType::BuilderBase => true,
        EntityType::MeleeBase => true,
        EntityType::RangedBase => true,
        EntityType::BuilderUnit => true,
        _ => false,
    }
}
