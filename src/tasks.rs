use std::collections::{HashMap, HashSet, VecDeque};

#[cfg(feature = "enable_debug")]
use model::{
    Color,
    ColoredVertex,
    DebugCommand,
    DebugData,
    DebugState,
};
use model::EntityType;

#[cfg(feature = "enable_debug")]
use crate::DebugInterface;
use crate::my_strategy::{Group, GroupState, Positionable, Role, Tile, Vec2i, World};
#[cfg(feature = "enable_debug")]
use crate::my_strategy::Vec2f;

pub const TARGET_BUILDERS_COUNT: usize = 60;

#[derive(Debug)]
pub struct TaskManager {
    next_task_id: usize,
    tasks: HashMap<usize, Task>,
    order: VecDeque<usize>,
    stats: TasksCount,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            next_task_id: 0,
            tasks: HashMap::new(),
            order: VecDeque::new(),
            stats: TasksCount::default(),
        }
    }

    pub fn stats(&self) -> &TasksCount {
        &self.stats
    }

    pub fn update(&mut self, world: &World, roles: &mut HashMap<i32, Role>,
                  groups: &mut HashMap<usize, Group>) {
        let mut done = HashSet::new();
        for task_id in self.order.iter() {
            let status = self.tasks.get_mut(&task_id).as_mut().unwrap().update(world, roles, groups);
            if !matches!(status, TaskStatus::Wait) {
                done.insert(*task_id);
            }
        }
        for task_id in done.iter() {
            match &self.tasks[task_id] {
                Task::HarvestResources(..) => self.stats.harvest_resources -= 1,
                Task::BuildBuilders => self.stats.build_builders -= 1,
                Task::BuildBuilding(v) => if v.entity_type == EntityType::House {
                    self.stats.build_house -= 1;
                }
                Task::GatherGroup(..) => self.stats.gather_group -= 1,
                Task::RepairBuildings => self.stats.repair_buildings -= 1,
                _ => (),
            }
        }
        self.order.retain(|v| !done.contains(v));
        self.tasks.retain(|v, _| !done.contains(v));
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&mut self, state: &DebugState, debug: &mut DebugInterface) {
        debug.send(DebugCommand::Add {
            data: DebugData::PlacedText {
                text: format!("Tasks:"),
                vertex: ColoredVertex {
                    world_pos: None,
                    screen_offset: Vec2f::new(50.0, state.window_size.y as f64 - 50.0 - 32.0 * 2.0).as_model(),
                    color: Color { a: 1.0, r: 1.0, g: 1.0, b: 1.0 },
                },
                alignment: 0.0,
                size: 26.0,
            },
        });
        for i in 0..self.order.len() {
            debug.send(DebugCommand::Add {
                data: DebugData::PlacedText {
                    text: format!("{}) {:?}", i, self.tasks[&self.order[i]]),
                    vertex: ColoredVertex {
                        world_pos: None,
                        screen_offset: Vec2f::new(70.0, state.window_size.y as f64 - 50.0 - (i + 3) as f64 * 32.0).as_model(),
                        color: Color { a: 1.0, r: 1.0, g: 1.0, b: 1.0 },
                    },
                    alignment: 0.0,
                    size: 26.0,
                },
            });
        }
    }

    pub fn push_front(&mut self, task: Task) {
        let task_id = self.insert_task(task);
        self.order.push_front(task_id);
    }

    pub fn push_back(&mut self, task: Task) {
        let task_id = self.insert_task(task);
        self.order.push_back(task_id);
    }

    fn insert_task(&mut self, task: Task) -> usize {
        let task_id = self.next_task_id;
        self.next_task_id += 1;
        match &task {
            Task::HarvestResources(..) => self.stats.harvest_resources += 1,
            Task::BuildBuilders => self.stats.build_builders += 1,
            Task::BuildBuilding(v) => match v.entity_type {
                EntityType::House => self.stats.build_house += 1,
                EntityType::Turret => self.stats.build_turret += 1,
                EntityType::BuilderBase => self.stats.build_builder_base += 1,
                EntityType::MeleeBase => self.stats.build_melee_base += 1,
                EntityType::RangedBase => self.stats.build_ranged_base += 1,
                _ => (),
            }
            Task::GatherGroup(..) => self.stats.gather_group += 1,
            Task::RepairBuildings => self.stats.repair_buildings += 1,
            _ => (),
        }
        self.tasks.insert(task_id, task);
        task_id
    }
}

#[derive(Default, Debug)]
pub struct TasksCount {
    pub harvest_resources: usize,
    pub build_builders: usize,
    pub build_house: usize,
    pub build_turret: usize,
    pub build_builder_base: usize,
    pub build_melee_base: usize,
    pub build_ranged_base: usize,
    pub gather_group: usize,
    pub repair_buildings: usize,
}

#[derive(Debug)]
pub enum Task {
    HarvestResources(HarvestResourcesTask),
    BuildBuilders,
    RepairBuildings,
    BuildBuilding(BuildBuildingTask),
    GatherGroup(GatherGroupTask),
    BuildUnits(BuildUnitsTask),
}

impl Task {
    pub fn harvest_resources() -> Self {
        Task::HarvestResources(HarvestResourcesTask::new())
    }

    pub fn build_building(entity_type: EntityType) -> Self {
        Self::BuildBuilding(BuildBuildingTask::new(entity_type))
    }

    pub fn gather_group(group_id: usize) -> Self {
        Self::GatherGroup(GatherGroupTask::new(group_id))
    }

    pub fn build_units(entity_type: EntityType, count: usize) -> Self {
        Self::BuildUnits(BuildUnitsTask::new(entity_type, count))
    }

    pub fn update(&mut self, world: &World, roles: &mut HashMap<i32, Role>, groups: &mut HashMap<usize, Group>) -> TaskStatus {
        match self {
            Self::HarvestResources(task) => task.update(world, roles),
            Self::BuildBuilders => build_builders(world, roles),
            Self::RepairBuildings => repair_buildings(world, roles),
            Self::BuildBuilding(task) => task.update(world, roles),
            Self::GatherGroup(task) => task.update(world, roles, groups),
            Self::BuildUnits(task) => task.update(world, roles),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum TaskStatus {
    Wait,
    Done,
    Fail,
}

#[derive(Debug)]
pub struct HarvestResourcesTask {
    assignments: HashMap<i32, i32>,
}

impl HarvestResourcesTask {
    pub fn new() -> Self {
        Self { assignments: HashMap::new() }
    }

    pub fn update(&mut self, world: &World, roles: &mut HashMap<i32, Role>) -> TaskStatus {
        let mut assigned_builders = HashSet::new();
        let mut used_positions = HashSet::new();
        for builder in world.my_builder_units() {
            if let Role::Harvester { position } = roles[&builder.id] {
                if world.is_attacked_by_opponents(position) {
                    roles.insert(builder.id, Role::None);
                } else {
                    used_positions.insert(position);
                    assigned_builders.insert(builder.id);
                }
            }
        }
        for (builder_id, resource_id) in self.assignments.iter() {
            if world.find_entity(*resource_id).is_none() {
                roles.insert(*builder_id, Role::None);
                assigned_builders.remove(builder_id);
            }
        }
        self.assignments.retain(|k, _| assigned_builders.contains(k));
        const SHIFTS: &[Vec2i] = &[Vec2i::only_y(-1), Vec2i::only_x(-1), Vec2i::only_x(1), Vec2i::only_y(1)];
        let mut harvester_positions = HashMap::new();
        for resource in world.resources() {
            for shift in SHIFTS {
                let position = resource.position() + *shift;
                if !used_positions.contains(&position)
                    && world.contains(position)
                    && matches!(world.get_tile(position), Tile::Empty)
                    && !harvester_positions.contains_key(&position)
                    && world.is_inside_protected_perimeter(position)
                    && !world.is_attacked_by_opponents(position) {
                    harvester_positions.insert(position, resource.id);
                }
            }
        }
        for builder in world.my_builder_units() {
            if matches!(roles[&builder.id], Role::None) {
                let nearest = harvester_positions.iter()
                    .min_by_key(|(position, resource_id)| {
                        (
                            builder.position().distance(**position)
                                - world.distance_to_nearest_opponent(**position).unwrap_or(world.map_size()),
                            **resource_id
                        )
                    })
                    .map(|(k, v)| (*k, *v));
                if let Some((position, resource_id)) = nearest {
                    self.assignments.insert(builder.id, resource_id);
                    roles.insert(builder.id, Role::Harvester { position });
                    harvester_positions.remove(&position);
                }
            }
        }
        TaskStatus::Wait
    }
}

fn build_builders(world: &World, roles: &mut HashMap<i32, Role>) -> TaskStatus {
    let mut builders = world.get_my_builder_units_count();
    let units_count = world.get_my_units_count();
    let properties = world.get_entity_properties(&EntityType::BuilderUnit);
    let cost = world.get_entity_cost(&EntityType::BuilderUnit);
    for entity in world.my_bases() {
        if matches!(entity.entity_type, EntityType::BuilderBase) {
            let role = if
                (builders < TARGET_BUILDERS_COUNT && builders < 2 * units_count / 3 || units_count / 3 < builders)
                && entity.active
                && (matches!(roles[&entity.id], Role::None) || matches!(roles[&entity.id], Role::UnitBuilder))
                && world.try_allocated_resource_and_population(cost, properties.population_use) {
                builders += 1;
                Role::UnitBuilder
            } else {
                Role::None
            };
            roles.insert(entity.id, role);
        }
    }
    TaskStatus::Wait
}

fn repair_buildings(world: &World, roles: &mut HashMap<i32, Role>) -> TaskStatus {
    let done: Vec<i32> = roles.iter()
        .filter_map(|(entity_id, role)| {
            match role {
                Role::BuildingRepairer { building_id } => world.find_entity(*building_id)
                    .map(|building| {
                        if building.health >= world.get_entity_properties(&building.entity_type).max_health {
                            Some(*entity_id)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(Some(*entity_id)),
                _ => None,
            }
        })
        .collect();
    for builder_id in done.into_iter() {
        roles.insert(builder_id, Role::None);
    }
    let assigned: HashSet<i32> = roles.values()
        .filter_map(|v| match v {
            Role::BuildingRepairer { building_id: base_id } => Some(*base_id),
            _ => None,
        })
        .collect();
    let mut buildings: Vec<(i32, i32)> = world.my_buildings()
        .filter_map(|building| {
            if assigned.contains(&building.id) {
                return None;
            }
            let damage = world.get_entity_properties(&building.entity_type).max_health - building.health;
            if damage <= 0 {
                return None;
            }
            Some((-damage, building.id))
        })
        .collect();
    buildings.sort();
    let mut harvesters = roles.values().filter(|v| matches!(v, Role::Harvester { .. })).count();
    for (_, building_id) in buildings.into_iter() {
        let building = world.get_entity(building_id);
        let mut candidates: Vec<(i32, i32)> = world.my_builder_units()
            .filter(|v| match roles[&v.id] {
                Role::None => true,
                Role::Harvester { .. } => harvesters > 0,
                _ => false,
            })
            .map(|v| (v.distance(building), v.id))
            .collect();
        if candidates.is_empty() {
            break;
        }
        candidates.sort();
        let cost = world.get_entity_cost(&building.entity_type);
        let need = (world.get_my_builder_units_count() / 10).max(1).min(cost as usize / 40);
        while candidates.len() > need {
            candidates.pop();
        }
        for i in 0..candidates.len().min(need) {
            let builder_id = candidates[i].1;
            harvesters -= matches!(roles[&builder_id], Role::Harvester { .. }) as usize;
            roles.insert(builder_id, Role::BuildingRepairer { building_id });
        }
    }
    TaskStatus::Wait
}

#[derive(Debug)]
pub struct BuildBuildingTask {
    entity_type: EntityType,
    resource_reserved: bool,
    place_locked: bool,
    position: Option<Vec2i>,
    builder_ids: Vec<i32>,
    building_id: Option<i32>,
}

impl BuildBuildingTask {
    pub fn new(entity_type: EntityType) -> Self {
        Self {
            entity_type,
            resource_reserved: false,
            place_locked: false,
            position: None,
            builder_ids: Vec::new(),
            building_id: None,
        }
    }

    pub fn update(&mut self, world: &World, roles: &mut HashMap<i32, Role>) -> TaskStatus {
        let properties = world.get_entity_properties(&self.entity_type);
        let cost = world.get_entity_cost(&self.entity_type);
        if self.building_id.is_none() && !self.resource_reserved {
            if !world.try_request_resources(cost) {
                return TaskStatus::Wait;
            }
            self.resource_reserved = true;
        }
        if let (Some(position), None) = (self.position, self.building_id) {
            if let Tile::Entity(entity_id) = world.get_tile(position) {
                if world.get_entity(entity_id).entity_type == self.entity_type {
                    self.building_id = Some(entity_id);
                    if self.resource_reserved {
                        world.release_requested_resource(cost);
                        self.resource_reserved = false;
                    }
                    world.unlock_square(position, properties.size);
                    self.place_locked = false;
                }
            }
        }
        if let Some(building_id) = self.building_id {
            if world.find_entity(building_id).is_none() {
                return self.fail(world, roles);
            }
        }
        self.builder_ids.retain(|v| world.contains_entity(*v));
        let need = get_builders_count_for(world, &self.entity_type, roles, self.builder_ids.len(), self.building_id);
        while self.builder_ids.len() > need {
            roles.insert(self.builder_ids.pop().unwrap(), Role::None);
        }
        if let (Some(position), None) = (self.position, self.building_id) {
            if !world.is_empty_square(position, properties.size) {
                self.position = None;
                world.unlock_square(position, properties.size);
                self.place_locked = false;
            }
        }
        if self.position.is_some() && need <= self.builder_ids.len() && self.building_id.is_none() {
            return TaskStatus::Wait;
        }
        if let Some(building_id) = self.building_id {
            if world.get_entity(building_id).active {
                for builder_id in self.builder_ids.iter() {
                    roles.insert(*builder_id, Role::None);
                }
                if self.resource_reserved {
                    world.release_requested_resource(cost);
                }
                return TaskStatus::Done;
            }
        }
        if self.position.is_none() {
            let size = properties.size;
            self.position = world.find_free_space_for(&self.entity_type);
            if let Some(position) = self.position {
                world.lock_square(position, size);
                self.place_locked = true;
            }
        }
        if self.position.is_some() && need > self.builder_ids.len() {
            let mut candidates: Vec<(Vec2i, i32)> = world.my_builder_units()
                .filter(|entity| {
                    if self.builder_ids.iter().find(|v| **v == entity.id).is_some() {
                        return false;
                    }
                    entity.active && match roles[&entity.id] {
                        Role::None => true,
                        Role::Harvester { .. } => true,
                        _ => false,
                    }
                })
                .map(|v| (v.position(), v.id))
                .collect();
            candidates.sort();
            for i in 0..(need - self.builder_ids.len()).min(candidates.len()) {
                self.builder_ids.push(candidates[i].1);
            }
        }
        if let Some(position) = self.position {
            for builder_id in self.builder_ids.iter() {
                if let Some(base_id) = self.building_id {
                    roles.insert(*builder_id, Role::BuildingRepairer { building_id: base_id });
                } else {
                    roles.insert(*builder_id, Role::BuildingBuilder { position, entity_type: self.entity_type.clone() });
                }
            }
        }
        TaskStatus::Wait
    }

    fn fail(&mut self, world: &World, roles: &mut HashMap<i32, Role>) -> TaskStatus {
        for builder_id in self.builder_ids.iter() {
            roles.insert(*builder_id, Role::None);
        }
        let properties = world.get_entity_properties(&self.entity_type);
        let cost = world.get_entity_cost(&self.entity_type);
        if self.resource_reserved {
            world.release_requested_resource(cost);
        }
        if let (Some(position), true) = (self.position, self.place_locked) {
            world.unlock_square(position, properties.size);
        }
        TaskStatus::Fail
    }
}

fn get_builders_count_for(world: &World, entity_type: &EntityType, roles: &HashMap<i32, Role>, current: usize, building_id: Option<i32>) -> usize {
    match entity_type {
        EntityType::Turret => 1,
        EntityType::Wall => 1,
        _ => {
            let properties = world.get_entity_properties(entity_type);
            let builder_properties = world.get_entity_properties(&EntityType::BuilderUnit);
            let building_health = building_id.map(|v| world.get_entity(v).health).unwrap_or(0);
            let base_time_to_build = (properties.initial_cost - building_health) as f32 / builder_properties.repair.as_ref().unwrap().power as f32;
            let harvesters = roles.values().map(|v| matches!(v, Role::Harvester { .. })).count() + current;
            let builders = world.get_my_entity_count_of(&EntityType::BuilderUnit);
            let unit_cost = (
                world.get_entity_cost(&EntityType::BuilderUnit)
                    + world.get_entity_cost(&EntityType::MeleeUnit)
                    + world.get_entity_cost(&EntityType::RangedUnit)
            ) as f32 / 3.0;
            let harvest_per_tick = builder_properties.attack.as_ref().unwrap().damage
                * world.get_entity_properties(&EntityType::Resource).resource_per_health;
            // base_time_to_build / max_builders * (harvesters - max_builders) * harvest_per_tick >= population_provide * unit_cost
            // base_time_to_build / max_builders * (harvesters - max_builders) >= (population_provide * unit_cost) / harvest_per_tick
            // base_time_to_build * harvesters / max_builders - base_time_to_build >= (population_provide * unit_cost) / harvest_per_tick
            // base_time_to_build * harvesters / max_builders >=(population_provide * unit_cost / harvest_per_tick + base_time_to_build
            // base_time_to_build * harvesters / (population_provide * unit_cost / harvest_per_tick + base_time_to_build) >= max_builders
            let max_builders = (base_time_to_build * harvesters as f32)
                / (base_time_to_build + properties.population_provide as f32 * unit_cost / harvest_per_tick as f32);
            (max_builders.round() as usize).min(2 * properties.size as usize).min(builders / 2).min(harvesters / 2).max(1)
        }
    }
}

#[derive(Debug)]
pub struct GatherGroupTask {
    group_id: usize,
}

impl GatherGroupTask {
    pub fn new(group_id: usize) -> Self {
        Self { group_id }
    }

    pub fn update(&mut self, world: &World, roles: &mut HashMap<i32, Role>, groups: &mut HashMap<usize, Group>) -> TaskStatus {
        if let Some(group) = groups.get_mut(&self.group_id) {
            if group.is_full() {
                group.set_state(GroupState::Ready);
                return TaskStatus::Done;
            }
            for unit in world.my_units() {
                if unit.active
                    && group.need_more_of(&unit.entity_type)
                    && (matches!(roles[&unit.id], Role::None) || matches!(roles[&unit.id], Role::Harvester { .. }))
                    && !(
                    matches!(unit.entity_type, EntityType::BuilderUnit)
                        && roles.values().filter(|v| matches!(v, Role::Harvester { .. })).count() <= 10
                ) {
                    group.add_unit(unit.id, unit.entity_type.clone());
                    roles.insert(unit.id, Role::GroupMember { group_id: self.group_id });
                }
            }
            for base in world.my_bases() {
                if let Some(build) = world.get_entity_properties(&base.entity_type).build.as_ref() {
                    let properties = world.get_entity_properties(&build.options[0]);
                    let cost = world.get_entity_cost(&build.options[0]);
                    if matches!(roles[&base.id], Role::None) || matches!(roles[&base.id], Role::UnitBuilder) {
                        if base.active
                            && group.need_more_of(&build.options[0])
                            && world.try_allocated_resource_and_population(cost, properties.population_use) {
                            roles.insert(base.id, Role::GroupSupplier { group_id: self.group_id });
                        }
                    } else if roles[&base.id] == (Role::GroupSupplier { group_id: self.group_id }) {
                        if !base.active
                            || !group.need_more_of(&build.options[0])
                            || !world.try_allocated_resource_and_population(cost, properties.population_use) {
                            roles.insert(base.id, Role::None);
                        }
                    }
                }
            }
            TaskStatus::Wait
        } else {
            TaskStatus::Fail
        }
    }
}

#[derive(Debug)]
pub struct BuildUnitsTask {
    entity_type: EntityType,
    left: usize,
}

impl BuildUnitsTask {
    pub fn new(entity_type: EntityType, count: usize) -> Self {
        Self { entity_type, left: count }
    }

    pub fn update(&mut self, world: &World, roles: &mut HashMap<i32, Role>) -> TaskStatus {
        for base in world.my_bases() {
            if let Some(build) = world.get_entity_properties(&base.entity_type).build.as_ref() {
                if !base.active || !build.options.iter().any(|v| *v == self.entity_type) {
                    continue;
                }
                let unit_properties = world.get_entity_properties(&self.entity_type);
                let cost = world.get_entity_cost(&self.entity_type);
                if matches!(roles[&base.id], Role::None) && world.try_allocated_resource_and_population(cost, unit_properties.population_use) {
                    roles.insert(base.id, Role::UnitBuilder);
                    self.left -= 1;
                }
            }
        }
        if self.left == 0 {
            TaskStatus::Done
        } else {
            TaskStatus::Wait
        }
    }
}
