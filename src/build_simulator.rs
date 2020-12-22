use model::{EntityProperties, EntityType};

pub struct BuildProperties {
    pub harvest_rate: i32,
    pub construct_rate: i32,
    pub transfer_ticks: i32,
    pub builder_cost: i32,
    pub start_costs: Vec<i32>,
    pub construction_costs: Vec<i32>,
    pub population_provide: Vec<i32>,
}

impl BuildProperties {
    pub fn new(transfer_ticks: i32, entity_properties: &Vec<EntityProperties>) -> Self {
        let builder_properties = &entity_properties[EntityType::BuilderUnit as usize];
        Self {
            harvest_rate: entity_properties[EntityType::Resource as usize].resource_per_health * builder_properties.attack.as_ref().unwrap().damage,
            construct_rate: builder_properties.repair.as_ref().unwrap().power,
            transfer_ticks,
            builder_cost: builder_properties.initial_cost,
            start_costs: vec![
                entity_properties[EntityType::House as usize].initial_cost,
                entity_properties[EntityType::RangedBase as usize].initial_cost,
            ],
            construction_costs: vec![
                entity_properties[EntityType::House as usize].max_health,
                entity_properties[EntityType::RangedBase as usize].max_health,
            ],
            population_provide: vec![
                entity_properties[EntityType::House as usize].population_provide,
                entity_properties[EntityType::RangedBase as usize].population_provide,
            ],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuildTask {
    None,
    Harvest,
    Build(u32),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Builder {
    pub task: BuildTask,
    pub ticks_to_start: i32,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Building {
    House,
    RangedBase,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Construction {
    pub id: u32,
    pub building: Building,
    pub need_resource: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildSimulator {
    tick: i32,
    resource: i32,
    population_provide: i32,
    buildings: Vec<usize>,
    builders: Vec<Builder>,
    next_building_id: u32,
    constructions: Vec<Construction>,
}

impl BuildSimulator {
    pub fn new(resource: i32, population_provide: i32, builders: Vec<Builder>, constructions: Vec<Construction>) -> Self {
        Self {
            tick: 0,
            resource,
            population_provide,
            buildings: vec![0, 0],
            builders,
            next_building_id: constructions.iter().map(|v| v.id).max().map(|v| v + 1).unwrap_or(0),
            constructions,
        }
    }

    pub fn tick(&self) -> i32 {
        self.tick
    }

    pub fn resource(&self) -> i32 {
        self.resource
    }

    pub fn population_provide(&self) -> i32 {
        self.population_provide
    }

    pub fn buildings(&self) -> &Vec<usize> {
        &self.buildings
    }

    pub fn builders(&self) -> &Vec<Builder> {
        &self.builders
    }

    pub fn constructions(&self) -> &Vec<Construction> {
        &self.constructions
    }

    pub fn assign(&mut self, builder_index: usize, task: BuildTask, properties: &BuildProperties) {
        if self.builders[builder_index].task == task {
            return;
        }
        self.builders[builder_index].task = task;
        self.builders[builder_index].ticks_to_start = properties.transfer_ticks;
    }

    pub fn build(&mut self, builder_index: usize, building: Building, properties: &BuildProperties) {
        let start_cost = properties.start_costs[building as usize];
        self.resource -= start_cost;
        let id = self.next_building_id;
        self.constructions.push(Construction {
            id,
            building,
            need_resource: properties.construction_costs[building as usize],
        });
        self.next_building_id += 1;
        self.assign(builder_index, BuildTask::Build(id), properties);
    }

    pub fn buy_builder(&mut self, properties: &BuildProperties) {
        self.resource -= properties.builder_cost;
        self.builders.push(Builder {
            task: BuildTask::None,
            ticks_to_start: 0,
        });
    }

    pub fn simulate(&mut self, properties: &BuildProperties) {
        let resource = &mut self.resource;
        let builders = &mut self.builders;
        let constructions = &mut self.constructions;
        for builder in builders.iter_mut() {
            if builder.ticks_to_start > 0 {
                builder.ticks_to_start -= 1;
                continue;
            }
            match &builder.task {
                BuildTask::Harvest => {
                    *resource += properties.harvest_rate;
                }
                BuildTask::Build(id) => {
                    constructions.iter_mut()
                        .find(|v| v.id == *id)
                        .map(|v| {
                            let spend = properties.construct_rate.min(*resource).min(v.need_resource);
                            v.need_resource -= spend;
                            *resource -= spend;
                        });
                }
                _ => (),
            }
        }
        for construction in self.constructions.iter() {
            if construction.need_resource == 0 {
                self.population_provide += properties.population_provide[construction.building as usize];
                self.buildings[construction.building as usize] += 1;
            }
        }
        self.constructions.retain(|v| v.need_resource > 0);
        for builder in self.builders.iter_mut() {
            let unassign = if let BuildTask::Build(id) = &builder.task {
                self.constructions.iter().all(|v| v.id != *id)
            } else {
                false
            };
            if unassign {
                builder.task = BuildTask::None;
            }
        }
        self.tick += 1;
    }
}
