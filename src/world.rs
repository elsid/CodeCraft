use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(feature = "enable_debug")]
use std::collections::HashSet;

use model::{
    Entity,
    EntityProperties,
    EntityType,
    Player,
    PlayerView,
};
#[cfg(feature = "enable_debug")]
use model::Color;

use crate::my_strategy::{is_entity_base, is_entity_unit, Map, position_to_index, Positionable, Rect, Tile, Vec2i, visit_range};
#[cfg(feature = "enable_debug")]
use crate::my_strategy::{debug, Vec2f};

pub struct World {
    my_id: i32,
    map_size: i32,
    fog_of_war: bool,
    entity_properties: Vec<EntityProperties>,
    max_tick_count: i32,
    max_pathfind_nodes: i32,
    current_tick: i32,
    players: Vec<Player>,
    entities: Vec<Entity>,
    entities_by_id: HashMap<i32, usize>,
    my_entities_count: Vec<usize>,
    map: RefCell<Map>,
    population_use: i32,
    population_provide: i32,
    start_position: Vec2i,
    grow_direction: Vec2i,
    base_size: RefCell<Option<i32>>,
    requested_resource: RefCell<i32>,
    allocated_resource: RefCell<i32>,
    allocated_population: RefCell<i32>,
    protected_radius: RefCell<Option<i32>>,
    player_power: Vec<i32>,
    is_attacked_by_opponent: Vec<bool>,
    #[cfg(feature = "enable_debug")]
    player_score_time_series: Vec<Vec<i32>>,
    #[cfg(feature = "enable_debug")]
    player_power_time_series: Vec<Vec<i32>>,
    #[cfg(feature = "enable_debug")]
    player_destroy_score_time_series: Vec<Vec<i32>>,
    #[cfg(feature = "enable_debug")]
    player_spent_resource: Vec<i32>,
    #[cfg(feature = "enable_debug")]
    player_units_history: Vec<HashSet<i32>>,
    #[cfg(feature = "enable_debug")]
    player_total_resource_time_series: Vec<Vec<i32>>,
    #[cfg(feature = "enable_debug")]
    map_resource_time_series: Vec<i32>,
}

impl World {
    pub fn new(player_view: &PlayerView) -> Self {
        let mut entity_properties: Vec<EntityProperties> = std::iter::repeat(EntityProperties::default())
            .take(player_view.entity_properties.len())
            .collect();
        for (entity_type, v) in player_view.entity_properties.iter() {
            entity_properties[entity_type.clone() as usize] = v.clone();
        }
        let first_builder_position = player_view.entities.iter()
            .find(|v| v.player_id == Some(player_view.my_id) && matches!(v.entity_type, EntityType::BuilderUnit)).unwrap()
            .position();
        let (start_position_x, grow_direction_x) = if first_builder_position.x() < player_view.map_size / 2 {
            (0, 1)
        } else {
            (player_view.map_size - 1, -1)
        };
        let (start_position_y, grow_direction_y) = if first_builder_position.y() < player_view.map_size / 2 {
            (0, 1)
        } else {
            (player_view.map_size - 1, -1)
        };
        Self {
            my_id: player_view.my_id,
            map_size: player_view.map_size,
            fog_of_war: player_view.fog_of_war,
            entity_properties,
            max_tick_count: player_view.max_tick_count,
            max_pathfind_nodes: player_view.max_pathfind_nodes,
            current_tick: player_view.current_tick,
            players: Vec::new(),
            entities: Vec::new(),
            entities_by_id: HashMap::new(),
            my_entities_count: std::iter::repeat(0)
                .take(player_view.entity_properties.len())
                .collect(),
            map: RefCell::new(Map::new(player_view.map_size as usize)),
            population_use: 0,
            population_provide: 0,
            start_position: Vec2i::new(start_position_x, start_position_y),
            grow_direction: Vec2i::new(grow_direction_x, grow_direction_y),
            base_size: RefCell::new(None),
            requested_resource: RefCell::new(0),
            allocated_resource: RefCell::new(0),
            allocated_population: RefCell::new(0),
            protected_radius: RefCell::new(None),
            player_power: std::iter::repeat(0).take(player_view.players.len()).collect(),
            is_attacked_by_opponent: std::iter::repeat(false).take((player_view.map_size * player_view.map_size) as usize).collect(),
            #[cfg(feature = "enable_debug")]
            player_score_time_series: std::iter::repeat(Vec::new()).take(player_view.players.len()).collect(),
            #[cfg(feature = "enable_debug")]
            player_power_time_series: std::iter::repeat(Vec::new()).take(player_view.players.len()).collect(),
            #[cfg(feature = "enable_debug")]
            player_destroy_score_time_series: std::iter::repeat(Vec::new()).take(player_view.players.len()).collect(),
            #[cfg(feature = "enable_debug")]
            player_spent_resource: std::iter::repeat(0).take(player_view.players.len()).collect(),
            #[cfg(feature = "enable_debug")]
            player_units_history: std::iter::repeat(HashSet::new()).take(player_view.players.len()).collect(),
            #[cfg(feature = "enable_debug")]
            player_total_resource_time_series: std::iter::repeat(Vec::new()).take(player_view.players.len()).collect(),
            #[cfg(feature = "enable_debug")]
            map_resource_time_series: Vec::new(),
        }
    }

    pub fn update(&mut self, player_view: &PlayerView) {
        self.current_tick = player_view.current_tick;
        self.players = player_view.players.clone();
        self.map.borrow_mut().update_with_actual(
            player_view.my_id,
            player_view.fog_of_war,
            &player_view.entities,
            &self.entity_properties,
        );
        if player_view.fog_of_war {
            for entity in player_view.entities.iter() {
                if let Some(existing) = self.entities_by_id.get(&entity.id).cloned() {
                    self.entities[existing] = entity.clone();
                }
            }
            for entity in self.entities.iter() {
                if entity.player_id == Some(self.my_id) {
                    self.entities_by_id.remove(&entity.id);
                }
            }
            self.entities.retain(|entity| entity.player_id != Some(player_view.my_id));
            self.map.borrow_mut().update_with_cached(&self.entities, &self.entity_properties);
            let map = self.map.borrow();
            let entity_properties = &self.entity_properties;
            self.entities.retain(|v| {
                map.find_inside_square(v.position(), entity_properties[v.entity_type.clone() as usize].size, |_, tile, _| {
                    match tile {
                        Tile::Entity(entity_id) => entity_id != v.id,
                        _ => true,
                    }
                }).is_none()
            });
            for entity in player_view.entities.iter() {
                if !self.entities_by_id.contains_key(&entity.id) {
                    self.entities.push(entity.clone());
                }
            }
        } else {
            self.entities = player_view.entities.clone();
        }
        self.entities_by_id = self.entities.iter().enumerate().map(|(n, v)| (v.id, n)).collect();
        for count in self.my_entities_count.iter_mut() {
            *count = 0;
        }
        let entities_count = &mut self.my_entities_count;
        for entity in self.entities.iter() {
            if entity.player_id == Some(self.my_id) {
                entities_count[entity.entity_type.clone() as usize] += 1;
            }
        }
        self.population_use = self.my_entities().map(|v| self.get_entity_properties(&v.entity_type).population_use).sum();
        self.population_provide = self.my_entities().map(|v| self.get_entity_properties(&v.entity_type).population_provide).sum();
        *self.base_size.borrow_mut() = None;
        *self.allocated_resource.borrow_mut() = 0;
        *self.allocated_population.borrow_mut() = 0;
        *self.protected_radius.borrow_mut() = None;
        for i in 0..self.players.len() {
            let player_id = self.players[i].id;
            self.player_power[i] = self.entities.iter()
                .filter(|v| v.player_id == Some(player_id))
                .map(|v| v.health * self.get_entity_properties(&v.entity_type).attack.as_ref().map(|v| v.damage).unwrap_or(0))
                .sum::<i32>();
        }
        for value in self.is_attacked_by_opponent.iter_mut() {
            *value = false;
        }
        for entity_index in 0..self.entities.len() {
            if matches!(self.entities[entity_index].entity_type, EntityType::BuilderUnit)
                || self.entities[entity_index].player_id == Some(self.my_id) {
                continue;
            }
            let properties = self.get_entity_properties(&self.entities[entity_index].entity_type);
            if let Some(attack) = properties.attack.as_ref() {
                let position = self.entities[entity_index].position();
                visit_range(position, properties.size, attack.attack_range + 3, &self.bounds(), |position| {
                    self.is_attacked_by_opponent[position_to_index(position, self.map_size as usize)] = true;
                });
            }
        }
        #[cfg(feature = "enable_debug")]
        for i in 0..self.players.len() {
            let player_id = self.players[i].id;
            let score = self.players[i].score;
            self.player_score_time_series[i].push(score);
            let power = self.entities.iter()
                .filter(|entity| entity.player_id == Some(player_id))
                .map(|entity| self.get_entity_properties(&entity.entity_type).attack.as_ref().map(|v| v.damage * entity.health).unwrap_or(0))
                .sum();
            self.player_power_time_series[i].push(power);
            let destroy_score = self.entities.iter()
                .filter(|entity| entity.player_id == Some(player_id))
                .map(|entity| self.get_entity_properties(&entity.entity_type).destroy_score)
                .sum();
            self.player_destroy_score_time_series[i].push(destroy_score);
            let mut entities_count: Vec<i32> = std::iter::repeat(0)
                .take(self.entity_properties.len())
                .collect();
            for entity in self.entities.iter() {
                if entity.player_id == Some(player_id) {
                    entities_count[entity.entity_type.clone() as usize] += 1;
                }
            }
            for entity in self.entities.iter() {
                if entity.player_id == Some(player_id) {
                    if self.player_units_history[i].insert(entity.id) {
                        self.player_spent_resource[i] += self.get_entity_properties(&entity.entity_type).initial_cost
                            + entities_count[entity.entity_type.clone() as usize];
                    }
                }
            }
            self.player_total_resource_time_series[i].push(self.player_spent_resource[i] + self.players[i].resource);
        }
        #[cfg(feature = "enable_debug")]
            self.map_resource_time_series.push(self.resources().map(|v| v.health).sum());
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
        &self.entity_properties[entity_type.clone() as usize]
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

    pub fn is_tile_locked(&self, position: Vec2i) -> bool {
        self.map.borrow().is_tile_locked(position)
    }

    pub fn is_tile_cached(&self, position: Vec2i) -> bool {
        self.map.borrow().is_tile_cached(position)
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

    pub fn grow_direction(&self) -> Vec2i {
        self.grow_direction
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

    pub fn entities(&self) -> &Vec<Entity> {
        &self.entities
    }

    pub fn entity_properties(&self) -> &Vec<EntityProperties> {
        &self.entity_properties
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
        self.my_entities()
            .filter(move |v| !self.get_entity_properties(&v.entity_type).can_move)
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

    pub fn is_empty_square(&self, position: Vec2i, size: i32) -> bool {
        self.map.borrow().find_inside_square(position, size, |_, tile, _| {
            !matches!(tile, Tile::Empty)
        }).is_none()
    }

    pub fn find_free_tile_nearby(&self, position: Vec2i, size: i32) -> Option<Vec2i> {
        self.map.borrow().find_neighbour(position, size, |_, tile, locked| {
            !locked && matches!(tile, Tile::Empty)
        })
    }

    pub fn find_nearest_free_tile_nearby_for_unit(&self, position: Vec2i, size: i32, unit_id: i32) -> Option<Vec2i> {
        let unit_position = self.get_entity(unit_id).position();
        let mut result = None;
        self.map.borrow()
            .find_neighbour(position, size, |tile_position, tile, locked| {
                if !locked && (matches!(tile, Tile::Empty) || tile == Tile::Entity(unit_id))
                    && result.map(|v| unit_position.distance(v) > unit_position.distance(tile_position)).unwrap_or(true) {
                    result = Some(tile_position);
                }
                false
            });
        result
    }

    pub fn find_free_space_for(&self, entity_type: &EntityType) -> Option<Vec2i> {
        let size = self.get_entity_properties(entity_type).size;
        let house = matches!(entity_type, EntityType::House);
        let fit = |map: &Map, position: Vec2i| -> bool {
            if !map.contains(position) || !map.contains(position + Vec2i::both(size)) {
                return false;
            }
            let has_place_for_entity = map.find_inside_square(position, size, |_, tile, locked| {
                locked || !matches!(tile, Tile::Empty)
            }).is_none();
            let has_space_around = map.find_on_square_border(position - Vec2i::both(1), size + 2, |_, tile, locked| {
                locked || (!house && !matches!(tile, Tile::Empty))
                    || (house && !matches!(tile, Tile::Empty) && !matches!(tile, Tile::Outside))
                    || matches!(self.get_tile_entity_type(tile), Some(EntityType::Resource))
            }).is_none();
            has_place_for_entity && has_space_around
        };
        let map = self.map.borrow();
        let start = if house {
            let x = if self.start_position.x() < self.map_size / 2 {
                0
            } else {
                self.map_size - 1
            };
            let y = if self.start_position.y() < self.map_size / 2 {
                0
            } else {
                self.map_size - 1
            };
            Vec2i::new(x, y)
        } else {
            self.start_position
        };
        if fit(&map, start) {
            return Some(start);
        }
        for radius in 1..self.get_protected_radius() {
            let result = map.find_on_square_border(
                start - Vec2i::both(radius),
                2 * radius + 1,
                |v, _, _| fit(&map, v),
            );
            if result.is_some() {
                return result;
            }
        }
        None
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

    pub fn my_population_provide(&self) -> i32 {
        self.population_provide - *self.allocated_population.borrow()
    }

    pub fn allocated_population(&self) -> i32 {
        *self.allocated_population.borrow()
    }

    pub fn try_allocated_resource_and_population(&self, resource: i32, population: i32) -> bool {
        if self.my_resource() < resource || self.my_population_provide() < population {
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

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, debug: &mut debug::Debug) {
        use std::collections::{btree_map, BTreeMap};

        debug.add_static_text(format!("Tick {}", self.current_tick));
        debug.add_static_text(format!("Players power: {:?}", (0..self.players.len()).map(|i| (self.players[i].id, self.player_power[i])).collect::<BTreeMap<_, _>>()));
        let allocated = self.allocated_resource();
        let requested = self.requested_resource();
        debug.add_static_text(format!("Resource: {} - {} a - {} r = {}", self.my_player().resource, allocated, requested, self.my_resource()));
        debug.add_static_text(format!("Population: {} - {} a = {}", self.population_use(), self.allocated_population(), self.my_population_provide()));
        let mut count_by_entity_type: BTreeMap<String, usize> = BTreeMap::new();
        for entity in self.my_entities() {
            match count_by_entity_type.entry(format!("{:?}", entity.entity_type)) {
                btree_map::Entry::Vacant(v) => {
                    v.insert(1);
                }
                btree_map::Entry::Occupied(mut v) => {
                    *v.get_mut() += 1;
                }
            }
        }
        debug.add_static_text(format!("Map resource: {} {}", self.resources().count(), self.resources().map(|v| v.health).sum::<i32>()));
        debug.add_static_text(String::from("My entities:"));
        for (entity_type, count) in count_by_entity_type.iter() {
            debug.add_static_text(format!("{}: {}", entity_type, count));
        }
        self.map.borrow().debug_update(debug);
        debug.add_world_line(
            self.start_position.center() - Vec2f::both(0.5),
            self.start_position.center() + Vec2f::both(0.5),
            Color { a: 1.0, r: 0.0, g: 0.0, b: 1.0 },
        );
        debug.add_world_line(
            self.start_position.center() - Vec2f::both(0.5).left(),
            self.start_position.center() + Vec2f::both(0.5).left(),
            Color { a: 1.0, r: 0.0, g: 0.0, b: 1.0 },
        );

        debug.add_time_series_i32(
            0,
            String::from("Players score"),
            self.player_score_time_series.iter().enumerate()
                .map(|(i, v)| (v, debug::get_player_color(1.0, self.players[i].id))),
        );
        debug.add_time_series_i32(
            1,
            String::from("Players power"),
            self.player_power_time_series.iter().enumerate()
                .map(|(i, v)| (v, debug::get_player_color(1.0, self.players[i].id))),
        );
        debug.add_time_series_i32(
            2,
            String::from("Players destroy score"),
            self.player_destroy_score_time_series.iter().enumerate()
                .map(|(i, v)| (v, debug::get_player_color(1.0, self.players[i].id))),
        );
        debug.add_time_series_i32(
            3,
            String::from("Players total resource"),
            self.player_total_resource_time_series.iter().enumerate()
                .map(|(i, v)| (v, debug::get_player_color(1.0, self.players[i].id))),
        );
        debug.add_time_series_i32(
            4,
            String::from("Map resource"),
            [(&self.map_resource_time_series, Color { a: 1.0, r: 0.0, g: 1.0, b: 0.0 })].iter().cloned(),
        );
    }

    pub fn get_my_entity_count_of(&self, entity_type: &EntityType) -> usize {
        self.my_entities_count[entity_type.clone() as usize]
    }

    pub fn get_my_units_count(&self) -> usize {
        self.my_entities_count.iter().enumerate()
            .filter(|(k, _)| self.entity_properties[*k].can_move)
            .map(|(_, v)| *v)
            .sum()
    }

    pub fn get_entity_cost(&self, entity_type: &EntityType) -> i32 {
        let properties = self.get_entity_properties(entity_type);
        properties.initial_cost + if properties.can_move {
            self.get_my_entity_count_of(entity_type) as i32
        } else {
            0
        }
    }

    pub fn is_attacked_by_opponents(&self, position: Vec2i) -> bool {
        self.is_attacked_by_opponent[position_to_index(position, self.map_size as usize)]
    }

    pub fn get_protected_radius(&self) -> i32 {
        if let Some(v) = *self.protected_radius.borrow() {
            return v;
        }
        let result = self.my_entities()
            .filter(|v| is_protected_entity_type(&v.entity_type))
            .map(|entity| {
                let properties = &self.get_entity_properties(&entity.entity_type);
                entity.center(properties.size).distance(self.start_position)
                    + properties.size / 2
                    + properties.sight_range
            })
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

    pub fn bounds(&self) -> Rect {
        Rect::new(Vec2i::zero(), Vec2i::both(self.map_size))
    }

    pub fn visit_map_square<F: FnMut(Vec2i, Tile, bool)>(&self, position: Vec2i, size: i32, f: F) {
        self.map.borrow().visit_square(position, size, f);
    }

    pub fn visit_map_range<F: FnMut(Vec2i, Tile, bool)>(&self, position: Vec2i, size: i32, range: i32, f: F) {
        self.map.borrow().visit_range(position, size, range, f)
    }

    pub fn find_in_map_range<F: FnMut(Vec2i, Tile, bool) -> bool>(&self, position: Vec2i, size: i32, range: i32, f: F) -> Option<Vec2i> {
        self.map.borrow().find_in_range(position, size, range, f)
    }

    pub fn get_tile_entity_type(&self, tile: Tile) -> Option<EntityType> {
        if let Tile::Entity(entity_id) = tile {
            return Some(self.get_entity(entity_id).entity_type.clone());
        }
        None
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
