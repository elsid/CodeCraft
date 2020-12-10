use std::collections::hash_map;
use std::collections::HashMap;

use model::{
    Action,
    EntityAction,
    EntityType,
    PlayerView,
};
#[cfg(feature = "enable_debug")]
use model::{Color, DebugState};

#[cfg(feature = "enable_debug")]
use crate::DebugInterface;
use crate::my_strategy::{Group, GroupState, is_protected_entity_type, Positionable, Role, Stats, Task, TaskManager, World};
#[cfg(feature = "enable_debug")]
use crate::my_strategy::{
    debug,
    Vec2f,
    Vec2i,
};

pub struct Bot {
    stats: Vec<(i32, Stats)>,
    roles: HashMap<i32, Role>,
    next_group_id: usize,
    groups: HashMap<usize, Group>,
    tasks: TaskManager,
    world: World,
    actions: HashMap<i32, EntityAction>,
    opening: bool,
}

impl Bot {
    pub fn new(world: World) -> Self {
        Self {
            next_group_id: 0,
            groups: HashMap::new(),
            roles: world.my_entities().map(|v| (v.id, Role::None)).collect(),
            stats: world.players().iter().map(|v| (v.id, Stats::new(v.id))).collect(),
            tasks: TaskManager::new(),
            world,
            actions: HashMap::new(),
            opening: true,
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
        self.world.debug_update(&mut debug);
        debug.add_static_text(format!("Opening: {}", self.opening));
        self.debug_update_groups(&mut debug);
        self.debug_update_entities(&mut debug);
        self.tasks.debug_update(&mut debug);
        debug.send(debug_interface);
    }

    fn update(&mut self, player_view: &PlayerView) {
        self.world.update(player_view);
        self.update_stats();
        self.update_roles();
        self.update_groups();
        self.update_tasks();
        self.update_group_targets();
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
        for group in self.groups.values_mut() {
            group.update(world);
        }
        self.groups.retain(|_, group| match group.state() {
            GroupState::New => true,
            _ => !group.is_empty(),
        });
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
                    .map(|role| (entity.id, role.get_action(entity, &self.world, &self.groups)))
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

    fn create_group(&mut self, need: HashMap<EntityType, usize>) -> usize {
        let group_id = self.next_group_id;
        self.next_group_id += 1;
        self.groups.insert(group_id, Group::new(group_id, need));
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
        for group in self.groups.values_mut() {
            if group.is_empty() {
                continue;
            }
            let defend = group.units_count() < group.need_count() ||
                self.world.get_my_entity_count_of(&EntityType::MeleeUnit) + self.world.get_my_entity_count_of(&EntityType::RangedUnit) < 15;
            let position = group.get_center(&self.world);
            let world = &self.world;
            if defend {
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
                    group.set_target(Some(target.position()));
                } else {
                    let target = self.world.my_turrets()
                        .min_by_key(|v| {
                            v.center(world.get_entity_properties(&v.entity_type).size).distance(position)
                        })
                        .map(|v| v.position())
                        .unwrap_or(self.world.start_position());
                    group.set_target(Some(target));
                }
            } else {
                if let Some(target) = self.world.opponent_entities()
                    .min_by_key(|v| (v.center(world.get_entity_properties(&v.entity_type).size).distance(position), v.id)) {
                    group.set_target(Some(target.position()));
                } else {
                    group.set_target(Some(position));
                }
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
                Color { a: 1.0, r: 1.0, g: 0.5, b: 0.0 },
            );
            debug.add_world_text(
                format!("Role: {:?}", self.roles.get(&entity.id)),
                position,
                Vec2f::only_y(-28.0),
                Color { a: 1.0, r: 1.0, g: 0.5, b: 0.0 },
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
                    Vec2f::only_y(-2.0 * 28.0),
                    Color { a: 1.0, r: 1.0, g: 0.5, b: 0.0 },
                );
            }
        }
    }

    #[cfg(feature = "enable_debug")]
    fn debug_update_groups(&self, debug: &mut debug::Debug) {
        for group in self.groups.values() {
            if group.is_empty() {
                continue;
            }
            let center = group.get_center(&self.world);
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
                Vec2f::zero(),
                Color { a: 0.9, r: 1.0, g: 0.5, b: 0.25 },
            );
        }
        debug.add_static_text(String::from("My groups:"));
        let mut group_ids: Vec<usize> = self.groups.keys().cloned().collect();
        group_ids.sort();
        for group_id in group_ids.iter() {
            let group = &self.groups[group_id];
            debug.add_static_text(format!("{}: has={:?} target={:?}", group.id(), group.has(), group.target()));
        }
    }
}

