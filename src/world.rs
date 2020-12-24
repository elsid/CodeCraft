use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use model::{
    Entity,
    EntityProperties,
    EntityType,
    Player,
    PlayerView,
};
#[cfg(feature = "enable_debug")]
use model::Color;

use crate::my_strategy::{Config, FindPathTarget, is_entity_base, is_entity_unit, Map, PathFinder, position_to_index, Positionable, ReachabilityMap, Rect, Stats, Tile, Vec2i, visit_neighbour, visit_range, visit_square};
#[cfg(feature = "enable_debug")]
use crate::my_strategy::debug;

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
    requested_resource: RefCell<i32>,
    allocated_resource: RefCell<i32>,
    allocated_population: RefCell<i32>,
    protected_radius: i32,
    player_power: Vec<i32>,
    is_attacked_by_opponent: Vec<bool>,
    last_player_activity: Vec<i32>,
    base_center: Vec2i,
    reachability_map: RefCell<ReachabilityMap>,
    known_map_resource: i32,
    predicted_map_resource: f32,
    is_passable: Vec<bool>,
    harvest_positions: Vec<Vec2i>,
    paths: RefCell<Vec<PathFinder>>,
    moves: RefCell<Vec<(Vec2i, Vec2i)>>,
    config: Config,
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
    #[cfg(feature = "enable_debug")]
    predicted_map_resource_time_series: Vec<i32>,
    #[cfg(feature = "enable_debug")]
    total_map_resource_time_series: Vec<i32>,
    #[cfg(feature = "enable_debug")]
    builders_time_series: Vec<i32>,
    #[cfg(feature = "enable_debug")]
    required_builders_time_series: Vec<i32>,
}

impl World {
    pub fn new(player_view: &PlayerView, config: Config) -> Self {
        Self {
            my_id: player_view.my_id,
            map_size: player_view.map_size,
            fog_of_war: player_view.fog_of_war,
            entity_properties: make_entity_properties_vec(&player_view.entity_properties),
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
            start_position: Vec2i::zero(),
            grow_direction: Vec2i::new(1, 1),
            requested_resource: RefCell::new(0),
            allocated_resource: RefCell::new(0),
            allocated_population: RefCell::new(0),
            protected_radius: 0,
            player_power: std::iter::repeat(0).take(player_view.players.len()).collect(),
            is_attacked_by_opponent: std::iter::repeat(false).take((player_view.map_size * player_view.map_size) as usize).collect(),
            last_player_activity: std::iter::repeat(player_view.current_tick).take(player_view.players.len()).collect(),
            base_center: Vec2i::zero(),
            reachability_map: RefCell::new(ReachabilityMap::new(player_view.map_size as usize)),
            known_map_resource: 0,
            predicted_map_resource: 0.0,
            is_passable: Vec::new(),
            harvest_positions: Vec::new(),
            paths: RefCell::new(Vec::new()),
            moves: RefCell::new(Vec::new()),
            config,
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
            #[cfg(feature = "enable_debug")]
            predicted_map_resource_time_series: Vec::new(),
            #[cfg(feature = "enable_debug")]
            total_map_resource_time_series: Vec::new(),
            #[cfg(feature = "enable_debug")]
            builders_time_series: Vec::new(),
            #[cfg(feature = "enable_debug")]
            required_builders_time_series: Vec::new(),
        }
    }

    pub fn update(&mut self, player_view: &PlayerView, stats: &mut Stats) {
        self.current_tick = player_view.current_tick;
        if !self.players.is_empty() {
            for i in 0..self.players.len() {
                if player_view.players[i].score != self.players[i].score || player_view.players[i].resource != self.players[i].resource {
                    self.last_player_activity[i] = player_view.current_tick;
                }
            }
        }
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
        self.population_provide = self.my_entities()
            .filter(|entity| entity.active)
            .map(|entity| self.get_entity_properties(&entity.entity_type).population_provide)
            .sum();
        *self.allocated_resource.borrow_mut() = 0;
        *self.allocated_population.borrow_mut() = 0;
        self.protected_radius = self.my_entities()
            .filter(|entity| is_protected_entity_type(&entity.entity_type))
            .map(|entity| {
                let properties = &self.get_entity_properties(&entity.entity_type);
                entity.center(properties.size).distance(self.start_position)
                    + properties.size / 2
                    + properties.sight_range
            })
            .max()
            .unwrap_or(1);
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
        let base_center = if !matches!(self.map.borrow().get_tile(self.base_center), Tile::Empty) {
            let mut base_center = None;
            let mut min_distance_to_start = std::i32::MAX;
            let mut min_distance_to_center = std::i32::MAX;
            visit_range(self.start_position, 1, self.protected_radius, &self.bounds(), |position| {
                if !matches!(self.map.borrow().get_tile(position), Tile::Empty) {
                    return;
                }
                let distance_to_start = self.start_position.distance(position);
                let distance_to_center = self.start_position.distance(Vec2i::both(self.map_size / 2));
                if (min_distance_to_start, min_distance_to_center) > (distance_to_start, distance_to_center) {
                    base_center = Some(position);
                    min_distance_to_start = distance_to_start;
                    min_distance_to_center = distance_to_center;
                }
            });
            base_center.unwrap_or(self.start_position)
        } else {
            self.base_center
        };
        let mut is_passable: Vec<bool> = std::iter::repeat(true)
            .take((self.map_size * self.map_size) as usize)
            .collect();
        for entity in self.entities.iter() {
            match &entity.entity_type {
                EntityType::BuilderUnit | EntityType::MeleeUnit | EntityType::RangedUnit => continue,
                _ => (),
            }
            let size = self.get_entity_properties(&entity.entity_type).size;
            visit_square(entity.position(), size, |position| {
                is_passable[position_to_index(position, self.map_size as usize)] = false;
            });
        }
        if self.base_center != base_center || self.is_passable != is_passable {
            stats.add_path_updates(1);
            self.reachability_map.borrow_mut().update(base_center, &is_passable);
            self.is_passable = is_passable;
            self.base_center = base_center;
        }
        self.known_map_resource = self.resources().map(|v| v.health).sum();
        let discovered_map_part = 1.0 - self.count_unknown_tiles() as f32 / (self.map_size * self.map_size) as f32;
        self.predicted_map_resource = self.known_map_resource as f32 / discovered_map_part - self.known_map_resource as f32;
        let mut harvest_positions = HashSet::new();
        visit_range(self.start_position, 1, self.protected_radius, &self.bounds(), |position| {
            if let Some(EntityType::Resource) = self.get_tile_entity_type(self.get_tile(position)) {
                visit_neighbour(position, 1, |position| {
                    if self.contains(position) && self.is_reachable_from_base(position) {
                        harvest_positions.insert(position);
                    }
                });
            }
        });
        self.harvest_positions = harvest_positions.into_iter().collect();
        self.paths.borrow_mut().clear();
        self.moves.borrow_mut().clear();
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
            self.map_resource_time_series.push(self.known_map_resource);
        #[cfg(feature = "enable_debug")]
            self.predicted_map_resource_time_series.push(self.predicted_map_resource as i32);
        #[cfg(feature = "enable_debug")]
            self.total_map_resource_time_series.push(self.known_map_resource + self.predicted_map_resource as i32);
        #[cfg(feature = "enable_debug")]
            self.builders_time_series.push(self.get_my_entity_count_of(&EntityType::BuilderUnit) as i32);
        #[cfg(feature = "enable_debug")]
            self.required_builders_time_series.push(self.get_max_required_builders_count() as i32);
    }

    pub fn my_id(&self) -> i32 {
        self.my_id
    }

    pub fn map_size(&self) -> i32 {
        self.map_size
    }

    pub fn fog_of_war(&self) -> bool {
        self.fog_of_war
    }

    pub fn current_tick(&self) -> i32 {
        self.current_tick
    }

    pub fn max_tick_count(&self) -> i32 {
        self.max_tick_count
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

    pub fn count_unknown_tiles(&self) -> usize {
        self.map.borrow().count_unknown_tiles()
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

    pub fn visit_free_tiles_nearby<F: FnMut(Vec2i)>(&self, position: Vec2i, size: i32, mut f: F) {
        self.map.borrow().visit_neighbour(position, size, |position, tile, locked| {
            if !locked && matches!(tile, Tile::Empty) {
                f(position);
            }
        });
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
                let entity_type = self.get_tile_entity_type(tile);
                locked
                    || matches!(entity_type, Some(EntityType::BuilderBase))
                    || matches!(entity_type, Some(EntityType::MeleeBase))
                    || matches!(entity_type, Some(EntityType::RangedBase))
                    || matches!(entity_type, Some(EntityType::House))
                    || matches!(entity_type, Some(EntityType::Turret))
                    || matches!(entity_type, Some(EntityType::Wall))
                    || (!house && matches!(tile, Tile::Outside))
            }).is_none();
            has_place_for_entity && has_space_around
        };
        let map = self.map.borrow();
        if fit(&map, self.start_position) {
            return Some(self.start_position);
        }
        for radius in 1..self.protected_radius() {
            let result = map.find_on_square_border(
                self.start_position - Vec2i::both(radius),
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

    pub fn force_allocate_resource(&self, amount: i32) {
        *self.allocated_resource.borrow_mut() += amount;
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

    fn population_space(&self) -> i32 {
        self.population_provide - self.population_use - *self.allocated_population.borrow()
    }

    pub fn try_allocated_resource_and_population(&self, resource: i32, population: i32) -> bool {
        if self.my_resource() < resource || self.population_space() < population {
            return false;
        }
        *self.allocated_resource.borrow_mut() += resource;
        *self.allocated_population.borrow_mut() += population;
        true
    }

    pub fn add_move(&self, src: Vec2i, dst: Vec2i) {
        if !matches!(self.get_tile_entity_type(self.get_tile(dst)), Some(EntityType::Resource)) {
            self.moves.borrow_mut().push((src, dst));
        }
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
        debug.add_static_text(format!("Players last activity: {:?}", (0..self.players.len()).map(|i| (self.players[i].id, self.last_player_activity[i])).collect::<BTreeMap<_, _>>()));
        let allocated = *self.allocated_resource.borrow();
        let requested = *self.requested_resource.borrow();
        debug.add_static_text(format!("Resource: {} - {} a - {} r = {}", self.my_player().resource, allocated, requested, self.my_resource()));
        debug.add_static_text(format!("Population: {} - {} - {} a = {}", self.population_provide, self.population_use, self.allocated_population.borrow(), self.population_space()));
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
        for path in self.paths.borrow().iter() {
            path.debug_update(debug);
        }
        debug.add_static_text(format!("Map resource: {} k + {} p = {}", self.known_map_resource, self.predicted_map_resource, self.known_map_resource as f32 + self.predicted_map_resource));
        debug.add_static_text(format!("Max required builders: {}", self.get_max_required_builders_count()));
        debug.add_static_text(String::from("My entities:"));
        for (entity_type, count) in count_by_entity_type.iter() {
            debug.add_static_text(format!("{}: {}", entity_type, count));
        }
        self.map.borrow().debug_update(debug);
        debug.add_world_cross(self.start_position.center(), 0.5, Color { a: 1.0, r: 0.0, g: 0.0, b: 1.0 });
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
            [
                (&self.map_resource_time_series, Color { a: 1.0, r: 0.0, g: 1.0, b: 0.0 }),
                (&self.predicted_map_resource_time_series, Color { a: 1.0, r: 0.0, g: 0.0, b: 1.0 }),
                (&self.total_map_resource_time_series, Color { a: 1.0, r: 1.0, g: 0.0, b: 0.0 }),
            ].iter().cloned(),
        );
        debug.add_time_series_i32(
            5,
            String::from("Builders"),
            [
                (&self.builders_time_series, Color { a: 1.0, r: 0.0, g: 1.0, b: 0.0 }),
                (&self.required_builders_time_series, Color { a: 1.0, r: 0.0, g: 0.0, b: 1.0 }),
            ].iter().cloned(),
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

    pub fn protected_radius(&self) -> i32 {
        self.protected_radius
    }

    pub fn is_inside_protected_perimeter(&self, position: Vec2i) -> bool {
        position.distance(self.start_position) <= self.protected_radius()
    }

    pub fn get_max_required_builders_count(&self) -> usize {
        let properties = self.get_entity_properties(&EntityType::BuilderUnit);
        let map_resource_estimate = self.known_map_resource as f32 + self.predicted_map_resource;
        let ticks_left = (self.max_tick_count - self.current_tick).max(1);
        ((map_resource_estimate / (properties.attack.as_ref().unwrap().damage * ticks_left) as f32).round().max(1.0) as usize)
            .min(self.harvest_positions.len())
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

    pub fn visit_map_rect<F: FnMut(Vec2i, Tile, bool)>(&self, rect: &Rect, f: F) {
        self.map.borrow().visit_rect(rect, f);
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

    pub fn find_shortest_path_next_position<T: FindPathTarget>(&self, start: Vec2i, target: &T, find_nearest: bool, damage: i32) -> Option<Vec2i> {
        self.find_shortest_path_next_position_and_cost(start, target, find_nearest, damage)
            .map(|(v, _)| v)
    }

    pub fn find_shortest_path_next_position_and_cost<T: FindPathTarget>(&self, start: Vec2i, target: &T, find_nearest: bool, damage: i32) -> Option<(Vec2i, i32)> {
        if target.has_reached(start) {
            return Some((start, 0));
        }
        let mut path = PathFinder::new(start, self.map_size as usize);
        path.find_with_a_star(target, find_nearest, damage, self);
        if path.path().is_empty() {
            return None;
        }
        if let Some(cost) = path.cost() {
            let result = Some((path.path()[0], cost));
            self.paths.borrow_mut().push(path);
            return result;
        }
        None
    }

    pub fn has_move_from(&self, position: Vec2i) -> bool {
        self.moves.borrow().iter().any(|(src, _)| *src == position)
    }

    pub fn has_move_to(&self, position: Vec2i) -> bool {
        self.moves.borrow().iter().any(|(_, dst)| *dst == position)
    }

    pub fn is_player_alive(&self, player_id: i32) -> bool {
        self.current_tick - self.last_player_activity[(player_id - 1) as usize] < self.config.min_player_inactive_ticks
    }

    pub fn get_player_position(&self, player_id: i32) -> Vec2i {
        let player_index = match self.my_id {
            1 => player_id,
            2 => match player_id {
                1 => 2,
                2 => 1,
                3 => 4,
                4 => 3,
                _ => 1,
            }
            3 => match player_id {
                1 => 3,
                2 => 4,
                3 => 1,
                4 => 2,
                _ => 1,
            }
            4 => match player_id {
                1 => 4,
                2 => 3,
                3 => 2,
                4 => 1,
                _ => 1,
            }
            _ => player_id,
        };
        let builder_base_size = self.get_entity_properties(&EntityType::BuilderBase).size;
        match player_index {
            1 => Vec2i::both(5),
            2 => Vec2i::both(self.map_size - builder_base_size - 5),
            3 => Vec2i::new(self.map_size - builder_base_size - 5, 5),
            4 => Vec2i::new(5, self.map_size - builder_base_size - 5),
            _ => Vec2i::both(self.map_size / 2),
        }
    }

    pub fn is_reachable_from_base(&self, position: Vec2i) -> bool {
        self.reachability_map.borrow().is_reachable(position)
    }

    pub fn harvest_positions(&self) -> &Vec<Vec2i> {
        &self.harvest_positions
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

pub fn make_entity_properties_vec(entity_properties: &HashMap<EntityType, EntityProperties>) -> Vec<EntityProperties> {
    let mut result: Vec<EntityProperties> = std::iter::repeat(EntityProperties::default())
        .take(entity_properties.len())
        .collect();
    for (entity_type, v) in entity_properties.iter() {
        result[entity_type.clone() as usize] = v.clone();
    }
    result
}
