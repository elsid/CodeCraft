use std::cell::RefCell;
use std::collections::hash_map;
use std::collections::HashMap;
use std::time::Instant;

use model::{
    Action,
    Entity,
    EntityAction,
    EntityType,
    PlayerView,
};
#[cfg(feature = "enable_debug")]
use model::{Color, DebugState};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

#[cfg(feature = "enable_debug")]
use crate::DebugInterface;
use crate::my_strategy::{build_builders, Config, EntityPlan, EntityPlanner, EntitySimulator, Group, GroupState, harvest_resources, is_active_entity_type, is_protected_entity_type, Positionable, Range, Rect, repair_buildings, Role, Stats, Task, TaskManager, Tile, Vec2i, World};
#[cfg(feature = "enable_debug")]
use crate::my_strategy::{
    debug,
    Vec2f,
};

#[derive(Debug)]
enum OpeningType {
    None,
    Round1,
    Round2,
}

pub struct Bot {
    stats: RefCell<Stats>,
    roles: HashMap<i32, Role>,
    next_group_id: u32,
    groups: Vec<Group>,
    tasks: TaskManager,
    world: World,
    actions: HashMap<i32, EntityAction>,
    opening: OpeningType,
    config: Config,
    entity_targets: HashMap<i32, Vec2i>,
    entity_planners: HashMap<i32, EntityPlanner>,
    rng: RefCell<StdRng>,
}

impl Drop for Bot {
    fn drop(&mut self) {
        let stats = self.stats.borrow().get_result();
        #[cfg(feature = "write_stats")]
            serde_json::to_writer(
            &mut std::fs::File::create(
                std::env::var("STATS").expect("STATS env is not found")
            ).unwrap(),
            &stats,
        ).unwrap();
        println!(
            "[{}] {} {} {} {:?} {:?} {} {}", self.world.current_tick(),
            stats.total_entity_plan_cost, stats.find_hidden_path_calls, stats.reachability_updates,
            stats.last_tick_duration, stats.max_tick_duration, stats.last_tick_entity_plan_cost,
            stats.max_tick_entity_plan_cost
        );
    }
}

impl Bot {
    pub fn new(player_view: &PlayerView, config: Config) -> Self {
        let seed = player_view.entities.iter()
            .map(|v| v.position.x as u64 + v.position.y as u64)
            .sum();
        Self {
            next_group_id: 0,
            groups: Vec::new(),
            roles: player_view.entities.iter()
                .filter(|v| v.player_id == Some(player_view.my_id))
                .map(|v| (v.id, Role::None)).collect(),
            stats: RefCell::new(Stats::default()),
            tasks: TaskManager::new(),
            actions: HashMap::new(),
            opening: if player_view.entities.iter()
                .any(|v| v.player_id == Some(player_view.my_id) && matches!(v.entity_type, EntityType::RangedBase)) {
                OpeningType::Round1
            } else {
                OpeningType::Round2
            },
            entity_targets: HashMap::new(),
            entity_planners: HashMap::new(),
            rng: RefCell::new(StdRng::seed_from_u64(seed)),
            world: World::new(player_view, config.clone()),
            config,
        }
    }

    pub fn get_action(&mut self, player_view: &PlayerView) -> Action {
        let start = Instant::now();
        self.update(player_view);
        let result = self.entity_actions();
        for (entity_id, entity_action) in result.iter() {
            self.actions.insert(*entity_id, entity_action.clone());
        }
        self.stats.borrow_mut().set_last_tick_duration(Instant::now() - start);
        Action { entity_actions: result }
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, state: &DebugState, debug_interface: &mut DebugInterface) {
        if self.world.current_tick() == 0 {
            debug_interface.send(model::DebugCommand::SetAutoFlush { enable: false });
        }
        let mut debug = debug::Debug::new(state);
        self.world.debug_update(&mut debug);
        debug.add_static_text(format!("Opening: {:?}", self.opening));
        self.debug_update_groups(&mut debug);
        self.debug_update_entities(&mut debug);
        debug.add_static_text(format!("Entity plans: {}", self.entity_planners.len()));
        for entity_planner in self.entity_planners.values() {
            entity_planner.debug_update(self.world.entity_properties(), &mut debug);
        }
        self.tasks.debug_update(&mut debug);
        debug.send(debug_interface);
    }

    fn update(&mut self, player_view: &PlayerView) {
        if player_view.current_tick == 0 && player_view.fog_of_war {
            self.world.update(&extend_player_view(player_view), &mut *self.stats.borrow_mut());
        } else {
            self.world.update(player_view, &mut *self.stats.borrow_mut());
        }
        self.update_roles();
        self.update_groups();
        self.update_tasks();
        self.update_group_targets();
        self.update_entity_plans();
        self.update_entity_targets();
    }

    fn update_roles(&mut self) {
        let world = &self.world;
        self.roles.retain(|id, _| world.contains_entity(*id));
        for entity in self.world.my_entities() {
            if let hash_map::Entry::Vacant(v) = self.roles.entry(entity.id) {
                let role = match &entity.entity_type {
                    EntityType::Turret => Role::Fighter,
                    _ => Role::None,
                };
                v.insert(role);
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

    fn update_tasks(&mut self) {
        if matches!(self.opening, OpeningType::None) || self.try_play_opening() {
            self.opening = OpeningType::None;
            self.try_gather_group();
            self.try_build_house();
            self.try_build_ranged_base();
            self.try_build_builder_base();
        }
        self.tasks.update(&self.world, &mut self.roles, &mut self.groups);
        harvest_resources(&self.world, &mut self.roles);
        repair_buildings(&self.world, &mut self.roles);
        if matches!(self.opening, OpeningType::None) {
            build_builders(&self.world, &mut self.roles);
        }
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
        match self.opening {
            OpeningType::Round1 => self.try_play_opening_for_round1(),
            OpeningType::Round2 => self.try_play_opening_for_round2(),
            _ => true,
        }
    }

    fn try_play_opening_for_round1(&mut self) -> bool {
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
            self.tasks.push_back(Task::build_units(EntityType::BuilderUnit, (self.world.population_provide() - self.world.population_use()) as usize));
        }
        if self.world.get_my_entity_count_of(&EntityType::RangedBase) == 0
            || self.world.get_my_units_count() >= 15
            || self.world.get_my_entity_count_of(&EntityType::MeleeUnit) == 0
            || self.world.get_my_entity_count_of(&EntityType::RangedUnit) == 0 {
            return true;
        }
        if self.world.my_resource() >= self.world.get_entity_cost(&EntityType::House) {
            self.tasks.push_back(Task::build_building(EntityType::House));
        }
        false
    }

    fn try_play_opening_for_round2(&mut self) -> bool {
        if self.world.current_tick() == 0 {
            {
                let shift_x = if self.world.grow_direction().x() > 0 {
                    0
                } else {
                    -self.world.get_entity_properties(&EntityType::House).size
                };
                let shift_y = if self.world.grow_direction().y() > 0 {
                    0
                } else {
                    -self.world.get_entity_properties(&EntityType::House).size
                };
                self.tasks.push_back(Task::clear_area(
                    self.world.start_position() + Vec2i::new(shift_x, shift_y),
                    self.world.get_entity_properties(&EntityType::House).size,
                ));
            }
            self.tasks.push_back(Task::build_units(
                EntityType::BuilderUnit,
                (self.world.population_provide() - self.world.population_use()) as usize,
            ));
        }
        if self.world.current_tick() > 300
            || self.world.get_my_entity_count_of(&EntityType::RangedBase) >= 1 {
            return true;
        }
        if self.tasks.stats().build_ranged_base == 0
            && self.world.get_my_entity_count_of(&EntityType::RangedBase) == 0
            && self.world.get_my_entity_count_of(&EntityType::House) >= 4 {
            if self.world.my_resource() >= self.world.get_entity_cost(&EntityType::RangedBase) {
                self.tasks.push_back(Task::build_building(EntityType::RangedBase));
            }
        } else if (self.tasks.stats().build_house == 0 || (self.tasks.stats().build_house < 2 && self.world.get_my_entity_count_of(&EntityType::House) >= 2))
            && self.world.population_provide() == self.world.population_use()
            && self.world.my_resource() >= self.world.get_entity_cost(&EntityType::House) {
            self.tasks.push_back(Task::build_building(EntityType::House));
            self.tasks.push_back(Task::build_units(
                EntityType::BuilderUnit,
                self.world.get_entity_properties(&EntityType::House).population_provide as usize,
            ));
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
            && self.world.get_my_entity_count_of(&EntityType::RangedBase) == 0
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
        for i in 0..self.groups.len() {
            if self.groups[i].is_empty() {
                continue;
            }
            let target = self.get_group_target(&self.groups[i]);
            self.groups[i].set_target(Some(target));
        }
    }

    fn get_group_target(&self, group: &Group) -> Vec2i {
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
                self.world.start_position() + self.world.grow_direction() * self.world.protected_radius() / 2
            }
        } else {
            if let Some(target) = self.world.opponent_entities()
                .min_by_key(|v| (v.center(world.get_entity_properties(&v.entity_type).size).distance(position), v.id)) {
                target.position()
            } else if self.world.fog_of_war() {
                self.world.players().iter()
                    .filter(|player| player.id != self.world.my_id() && self.world.is_player_alive(player.id))
                    .min_by_key(|player| (self.world.get_player_position(player.id).distance(group.position()) + player.score))
                    .map(|player| self.world.get_player_position(player.id))
                    .unwrap_or(position)
            } else {
                position
            }
        }
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
        if self.entity_planners.contains_key(&entity.id) {
            return None;
        }
        if let Role::GroupMember { group_id } = &self.roles[&entity.id] {
            let properties = self.world.get_entity_properties(&entity.entity_type);
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
                if let Some(target) = nearest_free_position {
                    return if self.world.is_tile_cached(target) {
                        self.stats.borrow_mut().add_find_hidden_path_calls(1);
                        self.world.find_shortest_path_next_position(
                            entity.position(),
                            &Range::new(target, properties.sight_range),
                            true,
                        )
                    } else {
                        Some(target)
                    };
                }
            }
        }
        None
    }

    fn update_entity_plans(&mut self) {
        let world = &self.world;
        self.entity_planners.retain(|entity_id, _| world.contains_entity(*entity_id));
        for planner in self.entity_planners.values_mut() {
            planner.reset();
        }
        let mut my_entities = Vec::new();
        let mut opponent_entities = Vec::new();
        for my_entity in self.world.my_entities() {
            if !is_active_entity_type(&my_entity.entity_type, self.world.entity_properties()) {
                continue;
            }
            if let Some(attack) = self.world.get_entity_properties(&my_entity.entity_type).attack.as_ref() {
                let mut has_opponents = false;
                for opponent_entity in self.world.opponent_entities() {
                    if !is_active_entity_type(&opponent_entity.entity_type, self.world.entity_properties()) {
                        continue;
                    }
                    let opponent_properties = self.world.get_entity_properties(&opponent_entity.entity_type);
                    if let Some(opponent_attack) = opponent_properties.attack.as_ref() {
                        let opponent_bounds = Rect::new(opponent_entity.position(), opponent_entity.position() + Vec2i::both(opponent_properties.size));
                        let distance = opponent_bounds.distance_to_position(my_entity.position());
                        if distance <= opponent_attack.attack_range.max(attack.attack_range) + self.config.engage_distance {
                            has_opponents = true;
                            opponent_entities.push(opponent_entity);
                        }
                    }
                }
                if has_opponents {
                    my_entities.push(my_entity);
                }
            }
        }
        if my_entities.is_empty() {
            return;
        }
        opponent_entities.sort_by_key(|entity| entity.id);
        opponent_entities.dedup_by_key(|entity| entity.id);
        let mut simulated_entities = 0;
        let opponent_simulators: Vec<(i32, EntitySimulator)> = opponent_entities.iter()
            .map(|entity| (entity.id, self.make_entity_simulator(entity, &mut simulated_entities)))
            .collect();
        let my_simulators: Vec<(i32, EntitySimulator)> = my_entities.iter()
            .map(|entity| (entity.id, self.make_entity_simulator(entity, &mut simulated_entities)))
            .collect();
        let simulated_entities_per_plan = simulated_entities as f32 / (my_entities.len() + opponent_entities.len()) as f32;
        let estimated_iteration_cost = 2.0 * simulated_entities as f32 - simulated_entities_per_plan;
        let entity_plan_max_transitions = (
            (self.config.entity_plan_max_total_cost - self.stats.borrow().total_entity_plan_cost()) as f32
                / (self.world.max_tick_count() - self.world.current_tick()) as f32
                / estimated_iteration_cost
        ).min(self.config.entity_plan_max_transitions as f32)
            .min(self.config.entity_plan_max_cost_per_tick as f32 / estimated_iteration_cost)
            .max(1.0)
            .round() as usize;
        let mut plans = Vec::new();
        let mut rng = self.rng.borrow_mut();
        let mut plan_cost = 0;
        for i in 0..opponent_entities.len() {
            let config = &self.config;
            let mut entity_planner = EntityPlanner::new(
                opponent_entities[i].player_id.unwrap(),
                opponent_entities[i].id,
                config.entity_plan_min_depth,
                config.entity_plan_max_depth,
            );
            let plan = Self::make_entity_plan(
                &opponent_simulators[i].1, world, entity_plan_max_transitions, &plans,
                &mut entity_planner, &mut plan_cost, &mut *rng,
            );
            if !plan.transitions.is_empty() {
                plans.push((opponent_entities[i].id, plan));
            }
        }
        for i in 0..my_entities.len() {
            let config = &self.config;
            let entity_planner = self.entity_planners.entry(my_entities[i].id)
                .or_insert_with(|| {
                    EntityPlanner::new(
                        my_entities[i].player_id.unwrap(),
                        my_entities[i].id,
                        config.entity_plan_min_depth,
                        config.entity_plan_max_depth,
                    )
                });
            let plan = Self::make_entity_plan(
                &my_simulators[i].1, world, entity_plan_max_transitions, &plans,
                entity_planner, &mut plan_cost, &mut *rng,
            );
            if !plan.transitions.is_empty() {
                plans.push((my_entities[i].id, plan));
            }
        }
        plans.sort_by_key(|(entity_id, plan)| (-plan.score, *entity_id));
        for i in 1..plans.len() {
            if let Some(entity_planner) = self.entity_planners.get_mut(&plans[i].0) {
                let simulator = &my_simulators.iter()
                    .find(|(entity_id, _)| *entity_id == plans[i].0)
                    .unwrap().1;
                let plan = Self::make_entity_plan(
                    simulator, world, entity_plan_max_transitions, &plans,
                    entity_planner, &mut plan_cost, &mut *rng,
                );
                if !plan.transitions.is_empty() {
                    plans[i].1 = plan;
                }
            }
        }
        self.stats.borrow_mut().add_entity_plan_cost(plan_cost);
    }

    fn make_entity_simulator(&self, entity: &Entity, simulated_entities: &mut usize) -> EntitySimulator {
        let properties = self.world.get_entity_properties(&entity.entity_type);
        let map_size = 2 * properties.sight_range;
        let shift = entity.position() - Vec2i::both(map_size / 2);
        let bounds = Rect::new(
            shift.highest(Vec2i::zero()),
            (shift + Vec2i::both(map_size)).lowest(Vec2i::both(self.world.map_size())),
        );
        let simulator = EntitySimulator::new(bounds, &self.world);
        *simulated_entities += simulator.entities().iter()
            .filter(|entity| is_active_entity_type(&entity.entity_type, self.world.entity_properties()))
            .count();
        simulator
    }

    fn make_entity_plan<R: Rng>(simulator: &EntitySimulator, world: &World, entity_plan_max_transitions: usize,
                                plans: &[(i32, EntityPlan)], entity_planner: &mut EntityPlanner,
                                plan_cost: &mut usize, rng: &mut R) -> EntityPlan {
        let transitions = entity_planner.update(
            world.map_size(),
            simulator.clone(),
            world.entity_properties(),
            entity_plan_max_transitions,
            plans,
            &mut *rng,
        );
        let active_entities = simulator.entities().iter()
            .filter(|entity| is_active_entity_type(&entity.entity_type, world.entity_properties()))
            .count();
        *plan_cost += transitions * active_entities;
        entity_planner.plan().clone()
    }

    #[cfg(feature = "enable_debug")]
    fn debug_update_entities(&self, debug: &mut debug::Debug) {
        for entity in self.world.my_entities() {
            let properties = self.world.get_entity_properties(&entity.entity_type);
            let position = Vec2f::from(entity.position()) + Vec2f::both(properties.size as f32) / 2.0;
            debug.add_world_text(
                format!("{} ({}, {}) {}", entity.id, entity.position.x, entity.position.y, entity.active),
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
                    Role::Harvester { resource_id } => debug.add_world_line(
                        position,
                        self.world.get_entity(*resource_id).position().center(),
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
            debug.add_static_text(format!(
                "{}) has={:?} position={:?} target={:?} state={:?}",
                group.id(), group.has(), group.position(), group.target(), group.state()
            ));
        }
    }
}

fn extend_player_view(player_view: &PlayerView) -> PlayerView {
    let mut result = player_view.clone();
    for player in player_view.players.iter() {
        if player.id != player_view.my_id {
            let properties = &player_view.entity_properties[&EntityType::BuilderBase];
            result.entities.push(Entity {
                player_id: Some(player.id),
                position: get_player_initial_builder_base_position(
                    player.id,
                    player_view.map_size,
                    properties.size,
                ).as_model(),
                entity_type: EntityType::BuilderBase,
                id: -player.id,
                health: properties.max_health,
                active: true,
            });
        }
    }
    result
}

fn get_player_initial_builder_base_position(player_id: i32, map_size: i32, builder_base_size: i32) -> Vec2i {
    match player_id {
        1 => Vec2i::both(5),
        2 => Vec2i::both(map_size - builder_base_size - 5),
        3 => Vec2i::new(map_size - builder_base_size - 5, 5),
        4 => Vec2i::new(5, map_size - builder_base_size - 5),
        _ => Vec2i::both(map_size / 2),
    }
}
