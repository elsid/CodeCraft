use std::collections::BinaryHeap;

use model::{EntityProperties, EntityType};
#[cfg(feature = "enable_debug")]
use model::Color;
use rand::Rng;
use rand::seq::SliceRandom;

use crate::my_strategy::{EntitySimulator, position_to_index, SimulatedEntity, SimulatedEntityAction, SimulatedEntityActionType, Vec2i, visit_range};
#[cfg(feature = "enable_debug")]
use crate::my_strategy::{
    debug,
    Vec2f,
};

#[derive(Default, Debug, Clone)]
pub struct EntityPlan {
    pub transitions: Vec<SimulatedEntityActionType>,
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
    pub action_type: SimulatedEntityActionType,
}

pub struct EntityPlanner {
    player_id: i32,
    entity_id: i32,
    states: Vec<State>,
    transitions: Vec<Transition>,
    plan: EntityPlan,
    min_depth: usize,
    max_depth: usize,
    optimal_final_state_index: Option<usize>,
}

impl EntityPlanner {
    pub fn new(player_id: i32, entity_id: i32, min_depth: usize, max_depth: usize) -> Self {
        Self {
            player_id,
            entity_id,
            states: Vec::new(),
            transitions: Vec::new(),
            plan: EntityPlan::default(),
            min_depth,
            max_depth,
            optimal_final_state_index: None,
        }
    }

    pub fn entity_id(&self) -> i32 {
        self.entity_id
    }

    pub fn plan(&self) -> &EntityPlan {
        &self.plan
    }

    pub fn reset(&mut self) {
        self.plan = EntityPlan::default();
    }

    pub fn update<R: Rng>(&mut self, map_size: i32, simulator: EntitySimulator,
                          entity_properties: &Vec<EntityProperties>, max_transitions: usize,
                          plans: &[(i32, EntityPlan)], rng: &mut R) -> usize {
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
        let mut transitions = 0;

        while let Some((score, state_index)) = frontier.pop() {
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
            if transitions >= max_transitions {
                continue;
            }
            let entity = if let Some(entity) = self.states[state_index].simulator.entities().iter()
                .find(|v| v.id == self.entity_id) {
                entity.clone()
            } else {
                continue;
            };
            let has_active_opponents = self.states[state_index].simulator.entities().iter()
                .any(|entity| {
                    entity.player_id.is_some() && entity.player_id != Some(self.player_id)
                        && is_active_entity_type(&entity.entity_type, entity_properties)
                });
            if !has_active_opponents {
                continue;
            }
            let other_actions = self.get_other_actions(&self.states[state_index], entity_properties, plans);
            let mut actions = Vec::new();
            Self::add_attack_actions(&entity, &self.states[state_index].simulator, entity_properties, &mut actions);
            Self::add_move_entity_actions(&entity, map_size, &mut actions);
            actions.push(SimulatedEntityActionType::None);
            actions.shuffle(rng);
            for action_type in actions.into_iter() {
                if transitions >= max_transitions {
                    break;
                }
                frontier.push(self.add_transition(action_type, other_actions.clone(), state_index, entity_properties, rng));
                transitions += 1;
            }
        }

        self.optimal_final_state_index = optimal_final_state_index;
        self.plan = optimal_final_state_index
            .map(|state_index| EntityPlan {
                score: max_score,
                transitions: self.reconstruct_sequence(state_index),
            })
            .unwrap_or_else(|| EntityPlan::default());

        transitions
    }

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(&self, entity_properties: &Vec<EntityProperties>, debug: &mut debug::Debug) {
        if self.plan.transitions.is_empty() {
            return;
        }
        let entity = self.states[0].simulator.entities().iter()
            .find(|v| v.id == self.entity_id)
            .unwrap();
        let mut position = entity.position;
        for action_type in self.plan.transitions.iter() {
            match action_type {
                SimulatedEntityActionType::MoveEntity { direction } => {
                    debug.add_world_line(
                        position.center(),
                        (position + *direction).center(),
                        Color { a: 1.0, r: 0.0, g: 0.0, b: 0.0 },
                    );
                    position += *direction;
                }
                SimulatedEntityActionType::Attack { target } => {
                    let target_entity = self.states[0].simulator.get_entity(*target);
                    let size = entity_properties[target_entity.entity_type.clone() as usize].size;
                    debug.add_world_line(
                        Vec2f::from(position.center()),
                        Vec2f::from(target_entity.position.center()),
                        Color { a: 1.0, r: 1.0, g: 0.0, b: 0.0 },
                    );
                    debug.add_world_line(
                        Vec2f::from(target_entity.position),
                        Vec2f::from(target_entity.position + Vec2i::both(size)),
                        Color { a: 1.0, r: 1.0, g: 0.0, b: 0.0 },
                    );
                    debug.add_world_line(
                        Vec2f::from(target_entity.position + Vec2i::only_x(size)),
                        Vec2f::from(target_entity.position + Vec2i::only_y(size)),
                        Color { a: 1.0, r: 1.0, g: 0.0, b: 0.0 },
                    );
                }
                _ => (),
            }
        }
        debug.add_static_text(format!(
            "Entity planner: states={} transitions={} score={} start={:?} plan={:?}",
            self.states.len(), self.transitions.len(), self.plan.score, entity.position, self.plan.transitions
        ));
    }

    fn get_other_actions(&self, state: &State, entity_properties: &Vec<EntityProperties>,
                         plans: &[(i32, EntityPlan)]) -> Vec<SimulatedEntityAction> {
        let mut result = Vec::new();
        for entity in state.simulator.entities() {
            if entity.id == self.entity_id || entity.player_id.is_none() {
                continue;
            }
            if let Some((_, plan)) = plans.iter().find(|(entity_id, _)| *entity_id == entity.id) {
                if state.depth < plan.transitions.len() {
                    result.push(SimulatedEntityAction {
                        entity_id: entity.id,
                        action_type: plan.transitions[state.depth].clone(),
                    });
                    continue;
                }
            }
            if is_active_entity_type(&entity.entity_type, entity_properties) {
                result.push(SimulatedEntityAction {
                    entity_id: entity.id,
                    action_type: SimulatedEntityActionType::AutoAttack,
                })
            }
        }
        result
    }

    fn add_attack_actions(entity: &SimulatedEntity, simulator: &EntitySimulator,
                          entity_properties: &Vec<EntityProperties>, actions: &mut Vec<SimulatedEntityActionType>) {
        let properties = &entity_properties[entity.entity_type.clone() as usize];
        if let Some(attack) = properties.attack.as_ref() {
            let map_size = simulator.map_width();
            let bounds = simulator.bounds();
            if simulator.entities().len() < (attack.attack_range * attack.attack_range) as usize {
                let entity_bounds = entity.bounds(entity_properties);
                for target in simulator.entities().iter() {
                    if target.id == entity.id {
                        continue;
                    }
                    if target.player_id.is_some() && target.player_id != entity.player_id
                        && target.bounds(entity_properties).distance(&entity_bounds) <= attack.attack_range {
                        actions.push(SimulatedEntityActionType::Attack { target: target.id });
                    }
                }
            } else {
                visit_range(entity.position, properties.size, attack.attack_range, &bounds, |position| {
                    if position == entity.position {
                        return;
                    }
                    if let Some(target_id) = simulator.tiles()[position_to_index(position - simulator.shift(), map_size)] {
                        let target = simulator.get_entity(target_id);
                        if target.player_id.is_some() && target.player_id != entity.player_id {
                            actions.push(SimulatedEntityActionType::Attack { target: target.id });
                        }
                    }
                });
            }
        }
    }

    fn add_move_entity_actions(entity: &SimulatedEntity, map_size: i32, actions: &mut Vec<SimulatedEntityActionType>) {
        if entity.position.x() + 1 < map_size {
            actions.push(SimulatedEntityActionType::MoveEntity { direction: Vec2i::only_x(1) });
        }
        if entity.position.y() + 1 < map_size {
            actions.push(SimulatedEntityActionType::MoveEntity { direction: Vec2i::only_y(1) });
        }
        if entity.position.x() > 0 {
            actions.push(SimulatedEntityActionType::MoveEntity { direction: Vec2i::only_x(-1) });
        }
        if entity.position.y() > 0 {
            actions.push(SimulatedEntityActionType::MoveEntity { direction: Vec2i::only_y(-1) });
        }
    }

    fn add_transition<R: Rng>(&mut self, action_type: SimulatedEntityActionType, mut actions: Vec<SimulatedEntityAction>,
                              state_index: usize, entity_properties: &Vec<EntityProperties>, rng: &mut R) -> (i32, usize) {
        let transition_index = self.transitions.len();
        self.transitions.push(Transition { state_index, action_type: action_type.clone() });
        let new_state_index = self.states.len();
        self.states.push(self.states[state_index].clone());
        let new_state = &mut self.states[new_state_index];
        new_state.transition = Some(transition_index);
        actions.push(SimulatedEntityAction {
            entity_id: self.entity_id,
            action_type,
        });
        new_state.simulator.simulate(entity_properties, &mut actions, rng);
        new_state.depth += 1;
        (
            self.get_score(&self.states[new_state_index].simulator),
            new_state_index,
        )
    }

    fn get_score(&self, simulator: &EntitySimulator) -> i32 {
        simulator.players().iter()
            .map(|player| {
                if player.id == self.player_id {
                    player.score + player.damage_done - player.damage_received
                } else {
                    player.damage_received - player.damage_done - player.score
                }
            })
            .sum()
    }

    fn reconstruct_sequence(&self, mut state_index: usize) -> Vec<SimulatedEntityActionType> {
        let mut result = Vec::new();
        while let Some(transition_index) = self.states[state_index].transition {
            let transition = &self.transitions[transition_index];
            result.push(transition.action_type.clone());
            state_index = transition.state_index;
        }
        result.reverse();
        result
    }
}

pub fn is_active_entity_type(entity_type: &EntityType, entity_properties: &Vec<EntityProperties>) -> bool {
    !matches!(*entity_type, EntityType::BuilderUnit)
        && entity_properties[entity_type.clone() as usize].attack.is_some()
}

#[cfg(test)]
mod tests {
    use model::{Entity, Player, PlayerView, Vec2I32};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use crate::my_strategy::{Config, examples, Rect, Stats, World};

    use super::*;

    fn new_player_view() -> PlayerView {
        let entity_properties = examples::entity_properties();
        PlayerView {
            my_id: 1,
            map_size: 80,
            fog_of_war: false,
            max_tick_count: 1000,
            max_pathfind_nodes: 1000,
            current_tick: 0,
            players: vec![
                Player {
                    id: 1,
                    score: 0,
                    resource: 0,
                },
                Player {
                    id: 2,
                    score: 0,
                    resource: 0,
                },
            ],
            entities: vec![
                Entity {
                    id: 1,
                    player_id: Some(1),
                    entity_type: EntityType::RangedUnit,
                    position: Vec2I32 { x: 30, y: 30 },
                    health: entity_properties[&EntityType::RangedUnit].max_health,
                    active: true,
                },
                Entity {
                    id: 2,
                    player_id: Some(2),
                    entity_type: EntityType::MeleeUnit,
                    position: Vec2I32 { x: 30, y: 35 },
                    health: 35,
                    active: true,
                },
            ],
            entity_properties,
        }
    }

    fn new_world() -> World {
        let player_view = new_player_view();
        let mut world = World::new(&player_view, Config::new());
        let mut stats = Stats::default();
        world.update(&player_view, &mut stats);
        world
    }

    #[test]
    fn plan() {
        let world = new_world();
        let simulator = EntitySimulator::new(Rect::new(Vec2i::both(20), Vec2i::both(40)), &world);
        let mut rng = StdRng::seed_from_u64(42);
        let mut planner = EntityPlanner::new(1, 1, 1, 4);
        let transitions = planner.update(world.map_size(), simulator, world.entity_properties(), 200, &[], &mut rng);
        assert!(!planner.plan().transitions.is_empty(), "iterations={}", transitions);
        assert_eq!((planner.plan().score, transitions), (40, 200), "{:?}", planner.plan().transitions);
    }
}
