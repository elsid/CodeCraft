use std::cell::RefCell;
use std::collections::hash_map;
use std::collections::HashMap;

use model::{
    Action,
    Entity,
    EntityAction,
    EntityType,
    PlayerView,
};
#[cfg(feature = "enable_debug")]
use model::{Color, DebugState};
use rand::rngs::StdRng;
use rand::SeedableRng;

#[cfg(feature = "enable_debug")]
use crate::DebugInterface;
use crate::my_strategy::{Config, EntityField, EntityPlanner, EntitySimulator, Field, Group, GroupField, GroupPlanner, GroupSimulator, GroupState, InfluenceField, is_protected_entity_type, Path, Positionable, Rect, Role, Stats, Task, TaskManager, Tile, Vec2i, visit_range, World};
#[cfg(feature = "enable_debug")]
use crate::my_strategy::{
    debug,
    Vec2f,
    visit_square,
};

pub struct Bot {
    stats: Vec<(i32, Stats)>,
    roles: HashMap<i32, Role>,
    next_group_id: u32,
    groups: Vec<Group>,
    tasks: TaskManager,
    world: World,
    actions: HashMap<i32, EntityAction>,
    opening: bool,
    config: Config,
    field: Field,
    influence_field: InfluenceField,
    group_fields: Vec<GroupField>,
    entity_fields: HashMap<i32, EntityField>,
    path: Path,
    entity_targets: HashMap<i32, Vec2i>,
    group_simulator: Option<GroupSimulator>,
    group_planners: Vec<GroupPlanner>,
    entity_planners: HashMap<i32, EntityPlanner>,
    rng: RefCell<StdRng>,
}

impl Bot {
    pub fn new(world: World, config: Config) -> Self {
        let seed = world.entities().iter()
            .map(|v| v.position.x as u64 + v.position.y as u64)
            .sum();
        Self {
            next_group_id: 0,
            groups: Vec::new(),
            roles: world.my_entities().map(|v| (v.id, Role::None)).collect(),
            stats: world.players().iter().map(|v| (v.id, Stats::new(v.id))).collect(),
            tasks: TaskManager::new(),
            actions: HashMap::new(),
            opening: true,
            field: Field::new(world.map_size(), config.clone()),
            influence_field: InfluenceField::new(world.map_size()),
            path: Path::new(world.map_size() as usize),
            world,
            config,
            group_fields: Vec::new(),
            entity_fields: HashMap::new(),
            entity_targets: HashMap::new(),
            group_simulator: None,
            group_planners: Vec::new(),
            entity_planners: HashMap::new(),
            rng: RefCell::new(StdRng::seed_from_u64(seed)),
        }
    }

    pub fn get_action(&mut self, player_view: &PlayerView) -> Action {
        self.update(player_view);
        let result = self.entity_actions();
        for (entity_id, entity_action) in result.iter() {
            self.actions.insert(*entity_id, entity_action.clone());
        }
        Action { entity_actions: result }
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, state: &DebugState, debug_interface: &mut DebugInterface) {
        if self.world.current_tick() == 0 {
            debug_interface.send(model::DebugCommand::SetAutoFlush { enable: false });
        }
        let mut debug = debug::Debug::new(state);
        let mut features: Vec<String> = Vec::new();
        #[cfg(feature = "use_group_field")]
            features.push(String::from("use_group_field"));
        #[cfg(feature = "use_entity_field")]
            features.push(String::from("use_entity_field"));
        #[cfg(feature = "use_group_planner")]
            features.push(String::from("use_group_planner"));
        #[cfg(feature = "use_entity_planner")]
            features.push(String::from("use_entity_planner"));
        debug.add_static_text(format!("Features: {:?}", features));
        self.world.debug_update(&mut debug);
        // self.field.debug_update(&mut debug);
        for i in 0..self.groups.len() {
            self.group_fields[i].debug_update(&mut debug);
            break;
        }
        self.path.debug_update(&mut debug);
        debug.add_static_text(format!("Opening: {}", self.opening));
        self.debug_update_groups(&mut debug);
        self.debug_update_entities(&mut debug);
        for entity_planner in self.entity_planners.values() {
            entity_planner.debug_update(self.world.entity_properties(), &mut debug);
        }
        self.tasks.debug_update(&mut debug);
        debug.send(debug_interface);
    }

    fn update(&mut self, player_view: &PlayerView) {
        self.world.update(player_view);
        self.update_stats();
        self.update_roles();
        self.update_groups();
        #[cfg(any(feature = "use_group_field", feature = "use_entity_field"))]
            self.field.update(&self.world);
        #[cfg(feature = "use_group_field")]
            self.update_group_fields();
        #[cfg(feature = "use_entity_field")]
            self.update_entity_fields();
        #[cfg(feature = "use_group_planner")]
            {
                self.update_group_simulator();
                self.update_group_planners();
            }
        self.update_tasks();
        self.update_group_targets();
        self.update_entity_targets();
        #[cfg(feature = "use_entity_planner")]
            self.update_entity_plans();
    }

    fn update_group_simulator(&mut self) {
        self.group_simulator = Some(GroupSimulator::new(&self.groups, self.config.segment_size, &self.world));
    }

    fn update_stats(&mut self) {
        let world = &self.world;
        for (_, stats) in self.stats.iter_mut() {
            stats.update(world);
        }
    }

    fn update_roles(&mut self) {
        let world = &self.world;
        self.roles.retain(|id, _| world.contains_entity(*id));
        for entity in self.world.my_entities() {
            if let hash_map::Entry::Vacant(v) = self.roles.entry(entity.id) {
                v.insert(Role::None);
            }
        }
        for role in self.roles.values_mut() {
            if role.is_temporary() {
                *role = Role::None;
            }
        }
    }

    fn update_groups(&mut self) {
        let world = &self.world;
        for group in self.groups.iter_mut() {
            group.update(world);
        }
        self.groups.retain(|group| match group.state() {
            GroupState::New => true,
            _ => !group.is_empty(),
        });
    }

    fn update_group_fields(&mut self) {
        let groups = &self.groups;
        self.group_fields.retain(|group_field| groups.iter().any(|group| group.id() == group_field.group_id()));
        for i in 0..self.groups.len() {
            self.group_fields[i].update(&self.field, &self.groups);
        }
    }

    fn update_group_planners(&mut self) {
        let groups = &self.groups;
        self.group_planners.retain(|group_planner| groups.iter().any(|group| group.id() == group_planner.group_id()));
        for i in 0..self.groups.len() {
            let group = &self.groups[i];
            if group.is_empty() || group.power() == 0 {
                continue;
            }
            self.group_planners[i].update(self.group_simulator.as_ref().unwrap().clone(), self.config.group_plan_max_iterations);
        }
    }

    fn update_entity_fields(&mut self) {
        let world = &self.world;
        self.entity_fields.retain(|k, _| world.contains_entity(*k));
        for entity in self.world.my_entities() {
            if matches!(entity.entity_type, EntityType::MeleeUnit) || matches!(entity.entity_type, EntityType::RangedUnit) {
                self.entity_fields.entry(entity.id)
                    .or_insert_with(|| EntityField::new(world.map_size()))
                    .update(entity, &self.field, world);
            }
        }
    }

    fn update_tasks(&mut self) {
        if !self.opening || self.try_play_opening() {
            self.opening = false;
            self.try_gather_group();
            self.try_build_house();
            self.try_build_ranged_base();
            self.try_build_builder_base();
        }
        self.tasks.update(&self.world, &mut self.roles, &mut self.groups);
    }

    fn entity_actions(&self) -> HashMap<i32, EntityAction> {
        self.world.my_entities()
            .filter_map(|entity| {
                self.roles.get(&entity.id)
                    .map(|role| (entity.id, role.get_action(entity, &self.world, &self.groups, &self.entity_targets, &self.entity_planners)))
            })
            .filter(|(entity_id, action)| {
                self.actions.get(&entity_id).map(|v| *v != *action).unwrap_or(true)
            })
            .collect()
    }

    fn try_play_opening(&mut self) -> bool {
        if self.world.current_tick() == 0 {
            let mut need = HashMap::new();
            let melee_units = self.world.get_my_entity_count_of(&EntityType::MeleeUnit);
            if melee_units > 0 {
                need.insert(EntityType::MeleeUnit, melee_units);
            }
            let ranged_units = self.world.get_my_entity_count_of(&EntityType::RangedUnit);
            if ranged_units > 0 {
                need.insert(EntityType::RangedUnit, ranged_units);
            }
            if !need.is_empty() {
                self.gather_group(need);
            }
        }
        if self.world.get_my_entity_count_of(&EntityType::RangedBase) == 0
            || self.world.get_my_units_count() >= 15
            || self.world.get_my_entity_count_of(&EntityType::MeleeUnit) == 0
            || self.world.get_my_entity_count_of(&EntityType::RangedUnit) == 0 {
            self.tasks.push_back(Task::BuildBuilders);
            return true;
        }
        if self.world.my_resource() >= self.world.get_entity_cost(&EntityType::House) {
            self.tasks.push_back(Task::build_building(EntityType::House));
        }
        if self.world.current_tick() == 0 {
            self.tasks.push_back(Task::build_units(EntityType::BuilderUnit, (self.world.population_provide() - self.world.population_use()) as usize));
            self.tasks.push_back(Task::RepairBuildings);
            self.tasks.push_back(Task::harvest_resources());
        }
        false
    }

    fn try_gather_group(&mut self) {
        if self.tasks.stats().gather_group == 0 {
            let mut need = HashMap::new();
            if self.world.my_ranged_bases().any(|v| v.active) {
                need.insert(EntityType::RangedUnit, 5);
            } else if self.world.my_melee_bases().any(|v| v.active) {
                need.insert(EntityType::MeleeUnit, 5);
            }
            if !need.is_empty() {
                self.gather_group(need);
            }
        }
    }

    fn gather_group(&mut self, need: HashMap<EntityType, usize>) {
        let group_id = self.create_group(need);
        self.tasks.push_front(Task::gather_group(group_id));
    }

    fn create_group(&mut self, need: HashMap<EntityType, usize>) -> u32 {
        let group_id = self.next_group_id;
        self.next_group_id += 1;
        let mut group = Group::new(group_id, need);
        group.update(&self.world);
        self.groups.push(group);
        self.group_fields.push(GroupField::new(group_id, self.world.map_size(), self.config.clone()));
        self.group_planners.push(GroupPlanner::new(group_id, self.config.group_plan_max_depth, self.config.segment_size));
        group_id
    }

    fn try_build_house(&mut self) {
        let capacity_left = self.world.population_provide() - self.world.population_use();
        if (self.tasks.stats().build_house as i32) < (self.world.population_use() / 10).max(1).min(3)
            && (capacity_left < 5 || self.world.population_use() * 100 / self.world.population_provide() > 90) {
            self.tasks.push_front(Task::build_building(EntityType::House));
        }
    }

    fn try_build_ranged_base(&mut self) {
        if self.tasks.stats().build_ranged_base == 0
            && (self.world.get_my_entity_count_of(&EntityType::RangedBase) as i32) < self.world.my_resource() / self.world.get_entity_cost(&EntityType::RangedBase) / 3
            && self.world.get_my_entity_count_of(&EntityType::BuilderUnit) > 0
            && self.world.my_resource() >= self.world.get_entity_cost(&EntityType::RangedBase) {
            self.tasks.push_front(Task::build_building(EntityType::RangedBase));
        }
    }

    fn try_build_builder_base(&mut self) {
        if self.tasks.stats().build_builder_base == 0
            && self.world.get_my_entity_count_of(&EntityType::BuilderBase) == 0
            && self.world.get_my_entity_count_of(&EntityType::BuilderUnit) > 0
            && self.world.my_resource() >= self.world.get_entity_cost(&EntityType::BuilderBase) {
            self.tasks.push_front(Task::build_building(EntityType::BuilderBase));
        }
    }

    fn update_group_targets(&mut self) {
        let mut busy: Vec<Rect> = Vec::new();
        for i in 0..self.groups.len() {
            if self.groups[i].is_empty() || self.groups[i].power() == 0 {
                continue;
            }
            let target = if cfg!(feature = "use_group_field") {
                self.get_group_target_by_group_field(&self.groups[i], &self.group_fields[i])
            } else if cfg!(feature = "use_group_planner") {
                self.get_group_target_by_planner(&self.groups[i], &self.group_planners[i])
            } else {
                Some(self.get_group_target_naive(&self.groups[i]))
            };
            self.groups[i].set_target(target);
        }
    }

    fn get_group_target_naive(&self, group: &Group) -> Vec2i {
        let position = group.position();
        let world = &self.world;
        if self.world.get_my_entity_count_of(&EntityType::MeleeUnit) + self.world.get_my_entity_count_of(&EntityType::RangedUnit) < 15 {
            if let Some(target) = self.world.opponent_entities()
                .filter(|v| {
                    world.is_inside_protected_perimeter(v.center(world.get_entity_properties(&v.entity_type).size))
                })
                .min_by_key(|entity| {
                    let properties = world.get_entity_properties(&entity.entity_type);
                    let entity_center = entity.center(properties.size);
                    let distance_to_my_entity = world.my_entities()
                        .filter(|v| is_protected_entity_type(&v.entity_type))
                        .map(|v| v.center(world.get_entity_properties(&v.entity_type).size).distance(entity_center))
                        .min();
                    (distance_to_my_entity, entity_center.distance(position), entity.id)
                }) {
                target.position()
            } else {
                self.world.my_turrets()
                    .min_by_key(|v| {
                        v.center(world.get_entity_properties(&v.entity_type).size).distance(position)
                    })
                    .map(|v| v.position())
                    .unwrap_or(self.world.start_position())
            }
        } else {
            if let Some(target) = self.world.opponent_entities()
                .min_by_key(|v| (v.center(world.get_entity_properties(&v.entity_type).size).distance(position), v.id)) {
                target.position()
            } else {
                position
            }
        }
    }

    fn get_group_target_by_group_field(&self, group: &Group, group_field: &GroupField) -> Option<Vec2i> {
        let (mut max_score, mut optimal_position, mut min_score_ratio) = group.target()
            .map(|target| (group_field.get_segment_position_score(target / self.config.segment_size), Some(target), self.config.group_min_score_ratio))
            .unwrap_or((-std::f32::MAX, None, 0.0));
        let bounds = Rect::new(Vec2i::zero(), Vec2i::both(self.world.map_size() / self.config.segment_size));
        let defend = self.world.get_my_entity_count_of(&EntityType::MeleeUnit) + self.world.get_my_entity_count_of(&EntityType::RangedUnit) < 15;
        let range = if self.opening || matches!(group.state(), GroupState::New) || defend {
            self.world.get_protected_radius()
        } else {
            2 * self.world.map_size()
        } / self.config.segment_size;
        visit_range(self.world.start_position() / self.config.segment_size, 1, range, &bounds, |segment_position| {
            if Some(segment_position) == optimal_position {
                return;
            }
            let score = group_field.get_segment_position_score(segment_position);
            if max_score < score && (max_score.signum() != score.signum() || score / max_score - 1.0 >= min_score_ratio) {
                max_score = score;
                optimal_position = Some(segment_position * self.config.segment_size);
                min_score_ratio = 0.0;
            }
        });
        optimal_position
    }

    fn get_group_target_by_planner(&self, group: &Group, group_planner: &GroupPlanner) -> Option<Vec2i> {
        group_planner.group_plan().transitions.first()
            .map(|v| group.position() + *v * self.config.segment_size)
    }

    fn update_entity_targets(&mut self) {
        let mut result: Vec<(i32, Vec2i)> = Vec::new();
        for entity in self.world.my_entities() {
            if let Some(target) = self.get_entity_target(entity, &result) {
                result.push((entity.id, target));
            }
        }
        self.entity_targets.clear();
        for (entity_id, target) in result.into_iter() {
            self.entity_targets.insert(entity_id, target);
        }
    }

    fn get_entity_target(&self, entity: &Entity, busy: &Vec<(i32, Vec2i)>) -> Option<Vec2i> {
        let properties = self.world.get_entity_properties(&entity.entity_type);
        if let Role::GroupMember { group_id } = &self.roles[&entity.id] {
            let opponent_nearby = self.world.find_in_map_range(entity.position(), properties.size, properties.sight_range, |_, tile, _| {
                if let Tile::Entity(entity_id) = tile {
                    let other = self.world.get_entity(entity_id);
                    (matches!(other.entity_type, EntityType::RangedUnit)
                        || matches!(other.entity_type, EntityType::MeleeUnit)
                        || matches!(other.entity_type, EntityType::Turret))
                        && other.player_id.map(|v| v != self.world.my_id()).unwrap_or(false)
                } else {
                    false
                }
            }).is_some();
            if !opponent_nearby {
                let group = self.groups.iter().find(|v| v.id() == *group_id).unwrap();
                if let Some(group_target) = group.target()
                    .filter(|target| target.distance(entity.position()) <= properties.sight_range) {
                    let mut min_distance = std::i32::MAX;
                    let mut nearest_free_position = None;
                    self.world.visit_map_range(group_target, properties.size, properties.sight_range, |position, tile, locked| {
                        if locked || busy.iter().any(|(_, v)| *v == position) {
                            return;
                        }
                        if let Tile::Entity(entity_id) = tile {
                            if entity_id != entity.id {
                                return;
                            }
                        }
                        let distance = position.distance(group_target);
                        if min_distance > distance {
                            min_distance = distance;
                            nearest_free_position = Some(position);
                        }
                    });
                    if let Some(nearest_free_position) = nearest_free_position {
                        return Some(nearest_free_position);
                    }
                }
            }
        }
        if cfg!(feature = "use_entity_field") {
            self.get_entity_target_by_entity_field(entity, busy)
        } else {
            None
        }
    }

    fn get_entity_target_naive(&self, unit: &Entity) -> Vec2i {
        let position = unit.position();
        let world = &self.world;
        if self.world.get_my_entity_count_of(&EntityType::MeleeUnit) + self.world.get_my_entity_count_of(&EntityType::RangedUnit) < 15 {
            if let Some(target) = self.world.opponent_entities()
                .filter(|v| {
                    world.is_inside_protected_perimeter(v.center(world.get_entity_properties(&v.entity_type).size))
                })
                .min_by_key(|entity| {
                    let properties = world.get_entity_properties(&entity.entity_type);
                    let entity_center = entity.center(properties.size);
                    let distance_to_my_entity = world.my_entities()
                        .filter(|v| is_protected_entity_type(&v.entity_type))
                        .map(|v| v.center(world.get_entity_properties(&v.entity_type).size).distance(entity_center))
                        .min();
                    (distance_to_my_entity, entity_center.distance(position), entity.id)
                }) {
                target.position()
            } else {
                self.world.my_turrets()
                    .min_by_key(|v| {
                        v.center(world.get_entity_properties(&v.entity_type).size).distance(position)
                    })
                    .map(|v| v.position())
                    .unwrap_or(self.world.start_position())
            }
        } else {
            if let Some(target) = self.world.opponent_entities()
                .min_by_key(|v| (v.center(world.get_entity_properties(&v.entity_type).size).distance(position), v.id)) {
                target.position()
            } else {
                position
            }
        }
    }

    fn get_entity_target_by_entity_field(&self, entity: &Entity, busy: &Vec<(i32, Vec2i)>) -> Option<Vec2i> {
        if let Some(entity_field) = self.entity_fields.get(&entity.id) {
            let (mut max_score, mut optimal_position, mut min_score_ratio) = self.entity_targets.get(&entity.id)
                .filter(|target| !busy.iter().any(|(_, v)| *v == **target))
                .map(|target| (entity_field.get_position_score(*target), Some(*target), self.config.entity_min_score_ratio))
                .unwrap_or((-std::f32::MAX, None, 0.0));
            let properties = self.world.get_entity_properties(&entity.entity_type);
            self.world.visit_map_range(entity.position(), properties.size, properties.sight_range, |position, tile, locked| {
                if locked || Some(position) == optimal_position || busy.iter().any(|(_, v)| *v == position) {
                    return;
                }
                if let Tile::Entity(entity_id) = tile {
                    if entity_id != entity.id {
                        return;
                    }
                }
                let score = entity_field.get_position_score(position);
                if max_score < score && (max_score.signum() != score.signum()
                    || (max_score.signum() > 0.0 && score / max_score - 1.0 >= min_score_ratio)
                    || (max_score.signum() < 0.0 && max_score / score - 1.0 >= min_score_ratio)) {
                    max_score = score;
                    optimal_position = Some(position);
                    min_score_ratio = 0.0;
                }
            });
            return optimal_position;
        }
        None
    }

    fn update_entity_plans(&mut self) {
        let world = &self.world;
        self.entity_planners.retain(|entity_id, _| world.contains_entity(*entity_id));
        for planner in self.entity_planners.values_mut() {
            planner.reset();
        }
        let mut plans = Vec::new();
        let mut simulators = Vec::new();
        let mut rng = self.rng.borrow_mut();
        for entity in self.world.my_units() {
            if !matches!(self.roles[&entity.id], Role::GroupMember { .. }) {
                continue;
            }
            let properties = self.world.get_entity_properties(&entity.entity_type);
            let in_battle = properties.attack.as_ref()
                .map(|attack| {
                    self.world.opponent_entities()
                        .any(|opponent| {
                            let opponent_properties = self.world.get_entity_properties(&opponent.entity_type);
                            if let Some(opponent_attack) = opponent_properties.attack.as_ref() {
                                let bounds = Rect::new(opponent.position(), opponent.position() + Vec2i::both(opponent_properties.size));
                                let distance = bounds.distance_to_position(entity.position());
                                distance <= opponent_attack.attack_range.max(attack.attack_range) + 1
                            } else {
                                false
                            }
                        })
                })
                .unwrap_or(false);
            if !in_battle {
                continue;
            }
            let config = &self.config;
            let planner = self.entity_planners.entry(entity.id)
                .or_insert_with(|| {
                    EntityPlanner::new(
                        entity.player_id.unwrap(),
                        entity.id,
                        config.entity_plan_min_depth,
                        config.entity_plan_max_depth,
                    )
                });
            let map_size = 2 * properties.sight_range;
            let shift = (entity.position() - Vec2i::both(map_size / 2))
                .lowest(Vec2i::both(self.world.map_size() - map_size))
                .highest(Vec2i::zero());
            let simulator = EntitySimulator::new(shift, map_size as usize, &self.world);
            planner.update(
                self.world.map_size(),
                simulator.clone(),
                self.world.entity_properties(),
                self.config.entity_plan_max_iterations,
                &Vec::new(),
                &mut *rng,
            );
            if !planner.plan().transitions.is_empty() {
                simulators.push(simulator);
                plans.push((entity.id, planner.plan().clone()));
            }
        }
        plans.sort_by_key(|(entity_id, plan)| (-plan.score, *entity_id));
        for i in 0..plans.len() {
            let planner = self.entity_planners.get_mut(&plans[i].0).unwrap();
            planner.update(
                self.world.map_size(),
                simulators[i].clone(),
                self.world.entity_properties(),
                self.config.entity_plan_max_iterations,
                &plans[0..i],
                &mut *rng,
            );
            if !planner.plan().transitions.is_empty() {
                plans[i].1 = planner.plan().clone();
            }
        }
    }

    #[cfg(feature = "enable_debug")]
    fn debug_update_entities(&self, debug: &mut debug::Debug) {
        for entity in self.world.my_entities() {
            let properties = self.world.get_entity_properties(&entity.entity_type);
            let position = Vec2f::from(entity.position()) + Vec2f::both(properties.size as f32) / 2.0;
            debug.add_world_text(
                format!("{} ({}, {})", entity.id, entity.position.x, entity.position.y),
                position,
                Vec2f::zero(),
                Color { a: 1.0, r: 0.7, g: 0.5, b: 0.2 },
            );
            debug.add_world_text(
                format!("Role: {:?}, target: {:?}", self.roles.get(&entity.id), self.entity_targets.get(&entity.id)),
                position,
                Vec2f::only_y(-32.0),
                Color { a: 1.0, r: 0.7, g: 0.5, b: 0.2 },
            );
            if let Some(role) = self.roles.get(&entity.id) {
                match role {
                    Role::Harvester { position: v, .. } => debug.add_world_line(
                        position,
                        v.center(),
                        Color { a: 1.0, r: 1.0, g: 0.0, b: 0.0 },
                    ),
                    _ => (),
                }
            }
            if let Some(action) = self.actions.get(&entity.id) {
                let mut text = String::new();
                if action.attack_action.is_some() {
                    text += "Attack ";
                }
                if action.build_action.is_some() {
                    text += "Build ";
                }
                if action.repair_action.is_some() {
                    text += "Repair ";
                }
                if let Some(move_action) = action.move_action.as_ref() {
                    debug.add_world_line(
                        position,
                        Vec2i::from(move_action.target.clone()).center(),
                        Color { a: 1.0, r: 0.0, g: 1.0, b: 0.0 },
                    );
                    text += "Move ";
                }
                debug.add_world_text(
                    format!("Action: {}", text),
                    position,
                    Vec2f::only_y(-2.0 * 32.0),
                    Color { a: 1.0, r: 0.7, g: 0.5, b: 0.2 },
                );
            }
        }
    }

    #[cfg(feature = "enable_debug")]
    fn debug_update_groups(&self, debug: &mut debug::Debug) {
        for group in self.groups.iter() {
            if group.is_empty() {
                continue;
            }
            let center = group.position();
            if let Some(target) = group.target() {
                debug.add_world_line(
                    center.center(),
                    target.center(),
                    Color { a: 0.75, r: 0.0, g: 0.0, b: 0.5 },
                );
            }
            debug.add_world_rectangle(
                Vec2f::from(group.get_bounds_min(&self.world)),
                Vec2f::from(group.get_bounds_max(&self.world)),
                Color { a: 0.15, r: 0.0, g: 0.0, b: 1.0 },
            );
            debug.add_world_text(
                format!("Group {}", group.id()),
                center.center(),
                Vec2f::only_y(32.0),
                Color { a: 1.0, r: 0.7, g: 0.5, b: 0.2 },
            );
        }
        debug.add_static_text(String::from("Groups:"));
        for i in 0..self.groups.len() {
            let group = &self.groups[i];
            let group_field = &self.group_fields[i];
            let mut min_score = std::f32::MAX;
            let mut max_score = -std::f32::MAX;
            visit_square(Vec2i::zero(), self.world.map_size() / self.config.segment_size, |segment_position| {
                let score = group_field.get_segment_position_score(segment_position);
                min_score = min_score.min(score);
                max_score = max_score.max(score);
            });
            debug.add_static_text(format!(
                "{}) has={:?} position={:?} target={:?} state={:?} power={} score={:?} min_score={:?} max_score={:?}",
                group.id(), group.has(), group.position(), group.target(), group.state(), group.power(),
                group.target().map(|v| group_field.get_segment_position_score(v / self.config.segment_size)),
                min_score, max_score
            ));
        }
    }
}
