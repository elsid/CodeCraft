use std::collections::BinaryHeap;

#[cfg(feature = "enable_debug")]
use model::Color;
use model::EntityProperties;
use rand::Rng;
use rand::seq::SliceRandom;

use crate::my_strategy::{EntityPlanner, EntitySimulator, SimulatedEntityAction, SimulatedEntityActionType};
#[cfg(feature = "enable_debug")]
use crate::my_strategy::debug;

#[derive(Default, Debug, Clone)]
pub struct BattlePlan {
    pub transitions: Vec<Vec<SimulatedEntityAction>>,
    pub score: i32,
}

#[derive(Clone, Debug)]
struct State {
    pub depth: usize,
    pub simulator: EntitySimulator,
    pub transition: Option<usize>,
}

#[derive(Clone, Debug)]
struct Transition {
    pub state_index: usize,
    pub actions: Vec<SimulatedEntityAction>,
}

pub struct BattlePlanner {
    player_ids: Vec<i32>,
    states: Vec<State>,
    transitions: Vec<Transition>,
    plan: BattlePlan,
    min_depth: usize,
    max_depth: usize,
    optimal_final_state_index: Option<usize>,
}

impl BattlePlanner {
    pub fn new(player_ids: Vec<i32>, min_depth: usize, max_depth: usize) -> Self {
        Self {
            player_ids,
            states: Vec::new(),
            transitions: Vec::new(),
            plan: BattlePlan::default(),
            min_depth,
            max_depth,
            optimal_final_state_index: None,
        }
    }

    pub fn plan(&self) -> &BattlePlan {
        &self.plan
    }

    pub fn reset(&mut self) {
        self.plan = BattlePlan::default();
    }

    pub fn update<R: Rng>(&mut self, map_size: i32, simulator: EntitySimulator,
                          entity_properties: &Vec<EntityProperties>, max_transitions: usize,
                          plans: &[Vec<SimulatedEntityAction>], rng: &mut R) -> usize {
        self.states.clear();
        self.transitions.clear();
        self.states.push(State {
            depth: 0,
            simulator,
            transition: None,
        });

        let mut frontier: BinaryHeap<(i32, usize)> = BinaryHeap::new();
        frontier.push((0, 0));

        let mut max_score = std::i32::MIN;
        let mut optimal_final_state_index = None;
        let mut iteration = 0;

        while let Some((score, state_index)) = frontier.pop() {
            iteration += 1;
            let depth = self.states[state_index].depth;
            if depth >= self.min_depth {
                if max_score < score {
                    max_score = score;
                    optimal_final_state_index = Some(state_index);
                }
                if depth >= self.max_depth {
                    continue;
                }
            }
            if self.transitions.len() >= max_transitions {
                continue;
            }
            let mut actions: Vec<(usize, i32, Vec<SimulatedEntityActionType>)> = self.states[state_index].simulator.entities().iter().enumerate()
                .filter(|(_, entity)| entity.player_id.is_some() || entity_properties[entity.entity_type.clone() as usize].attack.is_some())
                .map(|(index, entity)| (index, entity.id, Vec::new()))
                .collect();
            for (index, _, actions) in actions.iter_mut() {
                let entity = &self.states[state_index].simulator.entities()[*index];
                if entity.player_id.map(|v| self.player_ids.contains(&v)).unwrap_or(false) {
                    EntityPlanner::add_attack_actions(&entity, &self.states[state_index].simulator, entity_properties, actions);
                    EntityPlanner::add_move_entity_actions(&entity, map_size, actions);
                    actions.shuffle(rng);
                } else if depth < plans.len() {
                    actions.push(
                        plans[depth].iter()
                            .find(|action| action.entity_id == entity.id)
                            .map(|action| action.action_type.clone())
                            .unwrap_or(SimulatedEntityActionType::AttackInRange)
                    );
                } else {
                    actions.push(SimulatedEntityActionType::AttackInRange);
                }
            }
            let mut action_index = 0;
            while self.transitions.len() < max_transitions {
                frontier.push(self.add_transition(&actions, action_index, state_index, entity_properties, rng));
                action_index += 1;
            }
        }

        self.optimal_final_state_index = optimal_final_state_index;
        self.plan = optimal_final_state_index
            .map(|state_index| BattlePlan {
                score: max_score,
                transitions: self.reconstruct_sequence(state_index),
            })
            .unwrap_or_else(|| BattlePlan::default());

        iteration
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, entity_properties: &Vec<EntityProperties>, debug: &mut debug::Debug) {
        debug.add_static_text(format!(
            "Battle planner: states={} transitions={} score={} plan={:?}",
            self.states.len(), self.transitions.len(), self.plan.score, self.plan.transitions
        ));
    }

    fn add_transition<R: Rng>(&mut self, actions: &Vec<(usize, i32, Vec<SimulatedEntityActionType>)>,
                              action_index: usize, state_index: usize, entity_properties: &Vec<EntityProperties>,
                              rng: &mut R) -> (i32, usize) {
        let mut transition_actions = Vec::new();
        for (_, entity_id, action_types) in actions.iter() {
            transition_actions.push(SimulatedEntityAction {
                entity_id: *entity_id,
                action_type: if action_index < action_types.len() {
                    action_types[action_index].clone()
                } else if !action_types.is_empty() {
                    action_types[action_types.len() - 1].clone()
                } else {
                    SimulatedEntityActionType::AttackInRange
                },
            });
        }
        let transition_index = self.transitions.len();
        self.transitions.push(Transition { state_index, actions: transition_actions.clone() });
        let new_state_index = self.states.len();
        self.states.push(self.states[state_index].clone());
        let new_state = &mut self.states[new_state_index];
        new_state.transition = Some(transition_index);
        for action in transition_actions.into_iter() {
            new_state.simulator.add_action(action);
        }
        new_state.simulator.simulate(entity_properties, rng);
        new_state.depth += 1;
        (
            self.get_score(&self.states[new_state_index].simulator),
            new_state_index,
        )
    }

    fn get_score(&self, simulator: &EntitySimulator) -> i32 {
        simulator.players().iter()
            .map(|player| {
                if self.player_ids.contains(&player.id) {
                    player.score + player.damage_done - player.damage_received
                } else {
                    player.damage_received - player.damage_done - player.score
                }
            })
            .sum()
    }

    fn reconstruct_sequence(&self, mut state_index: usize) -> Vec<Vec<SimulatedEntityAction>> {
        let mut result = Vec::new();
        while let Some(transition_index) = self.states[state_index].transition {
            let transition = &self.transitions[transition_index];
            result.push(transition.actions.clone());
            state_index = transition.state_index;
        }
        result.reverse();
        result
    }
}
