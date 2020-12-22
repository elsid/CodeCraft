use std::collections::BinaryHeap;

use model::{EntityProperties, EntityType};

use crate::my_strategy::{Building, BuildProperties, BuildSimulator, BuildTask};

#[derive(Clone, Debug)]
pub enum BuildAction {
    Assign {
        builder_index: usize,
        task: BuildTask,
    },
    BuyBuilder,
    Build {
        builder_index: usize,
        building: Building,
    },
    Simulate {
        ticks: i32,
    },
}

#[derive(Default, Debug, Clone)]
pub struct BuildPlan {
    pub transitions: Vec<BuildAction>,
    pub score: i32,
}

#[derive(Clone, Debug)]
struct State {
    pub depth: usize,
    pub simulator: BuildSimulator,
    pub transition: Option<usize>,
}

#[derive(Clone, Debug)]
struct Transition {
    pub state_index: usize,
    pub action: BuildAction,
}

pub struct BuildPlanner {
    max_depth: usize,
    states: Vec<State>,
    transitions: Vec<Transition>,
    optimal_final_state_index: Option<usize>,
}

impl BuildPlanner {
    pub fn new(max_depth: usize) -> Self {
        Self {
            max_depth,
            states: Vec::new(),
            transitions: Vec::new(),
            optimal_final_state_index: None,
        }
    }

    pub fn update<F: FnMut(&BuildSimulator) -> bool>(&mut self, simulator: BuildSimulator, entity_properties: &Vec<EntityProperties>, max_transitions: usize, mut is_final: F) -> (usize, BuildPlan) {
        self.states.clear();
        self.transitions.clear();
        self.states.push(State {
            depth: 0,
            simulator,
            transition: None,
        });

        let properties = BuildProperties::new(5, entity_properties);
        let construction_places = vec![
            4 * entity_properties[EntityType::House as usize].size,
            4 * entity_properties[EntityType::RangedBase as usize].size,
        ];
        let builder_population_use = entity_properties[EntityType::BuilderUnit as usize].population_use;

        let mut frontier: BinaryHeap<(i32, usize)> = BinaryHeap::new();
        frontier.push((0, 0));

        let mut max_score = std::i32::MIN;
        let mut optimal_final_state_index = None;
        let mut iteration = 0;

        while let Some((score, state_index)) = frontier.pop() {
            iteration += 1;
            if is_final(&self.states[state_index].simulator) {
                if max_score < score {
                    max_score = score;
                    optimal_final_state_index = Some(state_index);
                }
                continue;
            }
            if self.states[state_index].depth >= self.max_depth || self.transitions.len() >= max_transitions {
                continue;
            }
            let mut actions = Vec::new();
            self.try_buy_builder(state_index, &properties, builder_population_use, max_transitions, &mut actions);
            if self.states[state_index].simulator.constructions().iter().all(|v| !matches!(v.building, Building::RangedBase)) {
                if self.states[state_index].simulator.buildings()[Building::RangedBase as usize] == 0
                    && properties.start_costs[Building::RangedBase as usize] <= self.states[state_index].simulator.resource() {
                    self.try_build(Building::RangedBase, state_index, &properties, max_transitions, &mut actions);
                } else {
                    self.try_build(Building::House, state_index, &properties, max_transitions, &mut actions);
                }
            }
            self.try_assign_to_harvest(state_index, max_transitions, &mut actions);
            self.try_assign_to_build(state_index, &construction_places, &properties, max_transitions, &mut actions);
            actions.push(BuildAction::Simulate { ticks: 5 });
            for action in actions.into_iter() {
                if let Some(transition) = self.add_transition(action, state_index, &properties) {
                    frontier.push(transition);
                }
            }
        }

        self.optimal_final_state_index = optimal_final_state_index;
        let plan = optimal_final_state_index
            .map(|state_index| BuildPlan {
                score: max_score,
                transitions: self.reconstruct_sequence(state_index),
            })
            .unwrap_or_else(|| BuildPlan::default());

        (iteration, plan)
    }

    fn try_buy_builder(&self, state_index: usize, properties: &BuildProperties, builder_population_use: i32, max_transitions: usize, actions: &mut Vec<BuildAction>) {
        if self.transitions.len() >= max_transitions {
            return;
        }
        if properties.builder_cost <= self.states[state_index].simulator.resource()
            && self.states[state_index].simulator.builders().len() as i32 + builder_population_use <= self.states[state_index].simulator.population_provide() {
            actions.push(BuildAction::BuyBuilder);
        }
    }

    fn try_build(&self, building: Building, state_index: usize, properties: &BuildProperties, max_transitions: usize, actions: &mut Vec<BuildAction>) {
        if self.transitions.len() >= max_transitions {
            return;
        }
        if properties.start_costs[building as usize] > self.states[state_index].simulator.resource() {
            return;
        }
        let mut builder_index = self.states[state_index].simulator.builders().iter()
            .enumerate()
            .find(|(_, v)| matches!(v.task, BuildTask::None))
            .map(|(n, _)| n);
        if builder_index.is_none() {
            builder_index = self.states[state_index].simulator.builders().iter()
                .enumerate()
                .find(|(_, v)| matches!(v.task, BuildTask::Harvest))
                .map(|(n, _)| n);
        }
        if let Some(builder_index) = builder_index {
            actions.push(BuildAction::Build { builder_index, building });
        }
    }

    fn try_assign_to_harvest(&self, state_index: usize, max_transitions: usize, actions: &mut Vec<BuildAction>) {
        if self.transitions.len() >= max_transitions {
            return;
        }
        self.states[state_index].simulator.builders().iter().enumerate()
            .find(|(_, builder)| matches!(builder.task, BuildTask::None))
            .map(|(builder_index, _)| actions.push(BuildAction::Assign { builder_index, task: BuildTask::Harvest }));
    }

    fn try_assign_to_build(&self, state_index: usize, construction_places: &Vec<i32>, properties: &BuildProperties, max_transitions: usize, actions: &mut Vec<BuildAction>) {
        if self.transitions.len() >= max_transitions {
            return;
        }
        if self.states[state_index].simulator.constructions().len() == 0 || self.states[state_index].simulator.resource() == 0 {
            return;
        }
        let harvesters = self.states[state_index].simulator.builders().iter()
            .filter(|builder| matches!(builder.task, BuildTask::Harvest))
            .count();
        if harvesters == 0 {
            return;
        }
        let builders = self.states[state_index].simulator.builders().iter()
            .filter(|builder| matches!(builder.task, BuildTask::Build(..)))
            .count();
        let need_resource: i32 = self.states[state_index].simulator.constructions().iter()
            .map(|construction| construction.need_resource)
            .sum();
        let resource_income = (harvesters - 1) as i32 * properties.harvest_rate;
        let resource_outcome = (builders + 1) as i32 * properties.construct_rate;
        let ticks = need_resource / properties.construct_rate + 1;
        if self.states[state_index].simulator.resource() + resource_income * ticks < resource_outcome * ticks {
            return;
        }
        for (builder_index, builder) in self.states[state_index].simulator.builders().iter().enumerate() {
            if matches!(builder.task, BuildTask::None) || matches!(builder.task, BuildTask::Harvest) && builder.ticks_to_start == 0 {
                for construction in self.states[state_index].simulator.constructions().iter() {
                    let assigned = self.states[state_index].simulator.builders().iter()
                        .filter(|builder| builder.task == BuildTask::Build(construction.id))
                        .count() as i32;
                    if assigned < construction_places[construction.building as usize] {
                        actions.push(BuildAction::Assign { builder_index, task: BuildTask::Build(construction.id) });
                        return;
                    }
                }
            }
        }
    }

    fn add_transition(&mut self, action: BuildAction, state_index: usize,
                      properties: &BuildProperties) -> Option<(i32, usize)> {
        let mut new_state = self.states[state_index].clone();
        let new_state_index = self.states.len();
        // println!("[{}] {} -> {} {:?} {} {:?}", new_state.simulator.tick(), state_index, new_state_index, action, new_state.simulator.resource(), new_state.simulator.constructions());
        match &action {
            BuildAction::BuyBuilder => {
                new_state.simulator.buy_builder(properties);
            }
            BuildAction::Build { builder_index, building } => {
                new_state.simulator.build(*builder_index, *building, properties);
            }
            BuildAction::Assign { builder_index, task } => {
                new_state.simulator.assign(*builder_index, task.clone(), properties);
            }
            BuildAction::Simulate { ticks } => {
                for _ in 0..*ticks {
                    new_state.simulator.simulate(properties);
                }
            }
        }
        if self.states.iter().any(|state| state.simulator == new_state.simulator) {
            return None;
        }
        let transition_index = self.transitions.len();
        new_state.transition = Some(transition_index);
        new_state.depth += 1;
        self.transitions.push(Transition { state_index, action });
        self.states.push(new_state);
        Some((
            self.get_score(new_state_index),
            new_state_index,
        ))
    }

    fn get_score(&self, state_index: usize) -> i32 {
        let state = &self.states[state_index];
        state.simulator.resource()
            + state.simulator.population_provide()
            - state.simulator.tick()
            + state.simulator.builders().len() as i32
    }

    fn reconstruct_sequence(&self, mut state_index: usize) -> Vec<BuildAction> {
        let mut result = Vec::new();
        let mut simulate = 0;
        while let Some(transition_index) = self.states[state_index].transition {
            let transition = &self.transitions[transition_index];
            if let BuildAction::Simulate { ticks } = &transition.action {
                simulate += *ticks;
            } else {
                if simulate > 0 {
                    result.push(BuildAction::Simulate { ticks: simulate });
                    simulate = 0;
                }
                result.push(transition.action.clone());
            }
            state_index = transition.state_index;
        }
        if simulate > 0 {
            result.push(BuildAction::Simulate { ticks: simulate });
        }
        result.reverse();
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::my_strategy::{Builder, examples, make_entity_properties_vec, Construction};

    use super::*;

    #[test]
    fn plan_until_5_builders() {
        let mut planner = BuildPlanner::new(100);
        let simulator = BuildSimulator::new(
            0,
            5,
            vec![Builder {
                task: BuildTask::None,
                ticks_to_start: 0,
            }],
            vec![],
        );
        let entity_properties = make_entity_properties_vec(&examples::entity_properties());
        let is_final = |simulator: &BuildSimulator| {
            simulator.builders().len() >= 5
        };
        let (iterations, plan) = planner.update(simulator, &entity_properties, 1000, is_final);
        assert!(!plan.transitions.is_empty(), "iterations={}", iterations);
        assert_eq!(plan.score, -21, "{:?}", plan.transitions);
    }

    #[test]
    fn plan_until_first_construction() {
        let mut planner = BuildPlanner::new(100);
        let simulator = BuildSimulator::new(
            0,
            5,
            vec![
                Builder {
                    task: BuildTask::None,
                    ticks_to_start: 0,
                },
                Builder {
                    task: BuildTask::None,
                    ticks_to_start: 0,
                },
                Builder {
                    task: BuildTask::None,
                    ticks_to_start: 0,
                },
                Builder {
                    task: BuildTask::None,
                    ticks_to_start: 0,
                },
                Builder {
                    task: BuildTask::None,
                    ticks_to_start: 0,
                },
            ],
            vec![],
        );
        let entity_properties = make_entity_properties_vec(&examples::entity_properties());
        let is_final = |simulator: &BuildSimulator| {
            simulator.constructions().len() >= 1
        };
        let (iterations, plan) = planner.update(simulator, &entity_properties, 1000, is_final);
        assert!(!plan.transitions.is_empty(), "iterations={}", iterations);
        assert_eq!(plan.score, -5, "{:?}", plan.transitions);
    }

    #[test]
    fn plan_until_first_house() {
        let mut planner = BuildPlanner::new(100);
        let simulator = BuildSimulator::new(
            0,
            0,
            vec![
                Builder {
                    task: BuildTask::None,
                    ticks_to_start: 0,
                },
            ],
            vec![
                Construction {
                    id: 0,
                    building: Building::House,
                    need_resource: 1,
                }
            ],
        );
        let entity_properties = make_entity_properties_vec(&examples::entity_properties());
        let is_final = |simulator: &BuildSimulator| {
            simulator.buildings()[Building::House as usize] >= 1
        };
        let (iterations, plan) = planner.update(simulator, &entity_properties, 10000, is_final);
        assert!(!plan.transitions.is_empty(), "iterations={}", iterations);
        assert_eq!(plan.score, -6, "{:?}", &plan.transitions[0..plan.transitions.len().min(10)]);
    }

    #[test]
    fn plan_until_first_house_from_start() {
        let mut planner = BuildPlanner::new(100);
        let simulator = BuildSimulator::new(
            0,
            5,
            vec![
                Builder {
                    task: BuildTask::None,
                    ticks_to_start: 0,
                },
            ],
            vec![],
        );
        let entity_properties = make_entity_properties_vec(&examples::entity_properties());
        let is_final = |simulator: &BuildSimulator| {
            simulator.buildings()[Building::House as usize] >= 1
        };
        let (iterations, plan) = planner.update(simulator, &entity_properties, 1000, is_final);
        assert!(!plan.transitions.is_empty(), "iterations={}", iterations);
        assert_eq!(plan.score, -35, "{:?}", &plan.transitions);
    }

    #[test]
    fn plan_until_second_house_from_start() {
        let mut planner = BuildPlanner::new(200);
        let simulator = BuildSimulator::new(
            0,
            5,
            vec![
                Builder {
                    task: BuildTask::None,
                    ticks_to_start: 0,
                },
            ],
            vec![],
        );
        let entity_properties = make_entity_properties_vec(&examples::entity_properties());
        let is_final = |simulator: &BuildSimulator| {
            simulator.buildings()[Building::House as usize] >= 2
        };
        let (iterations, plan) = planner.update(simulator, &entity_properties, 1000, is_final);
        assert!(!plan.transitions.is_empty(), "iterations={}", iterations);
        assert_eq!(plan.score, -49, "{:?}", &plan.transitions);
    }

    #[test]
    fn plan_until_ranged_base() {
        let mut planner = BuildPlanner::new(1000);
        let simulator = BuildSimulator::new(
            0,
            5,
            vec![
                Builder {
                    task: BuildTask::None,
                    ticks_to_start: 0,
                },
            ],
            vec![],
        );
        let entity_properties = make_entity_properties_vec(&examples::entity_properties());
        let is_final = |simulator: &BuildSimulator| {
            simulator.buildings()[Building::RangedBase as usize] >= 1
        };
        let (iterations, plan) = planner.update(simulator, &entity_properties, 1000, is_final);
        assert!(!plan.transitions.is_empty(), "iterations={}", iterations);
        assert_eq!(plan.score, 213, "{:?}", &plan.transitions);
    }
}
