use std::collections::BinaryHeap;

use itertools::Itertools;
use model::{EntityProperties, EntityType};
use rand::Rng;
use rand::seq::SliceRandom;

use crate::my_strategy::{index_to_position, position_to_index, Positionable, Rect, Tile, Vec2i, visit_reversed_shortest_path, visit_square_with_bounds, World};

#[derive(Clone, Debug)]
pub struct SimulatedPlayer {
    pub id: i32,
    pub score: i32,
    pub damage_done: i32,
    pub damage_received: i32,
}

#[derive(Clone, Debug)]
pub struct SimulatedEntity {
    pub id: i32,
    pub entity_type: EntityType,
    pub position: Vec2i,
    pub player_id: Option<i32>,
    pub health: i32,
    pub active: bool,
    pub available: bool,
}

impl SimulatedEntity {
    pub fn bounds(&self, entity_properties: &Vec<EntityProperties>) -> Rect {
        let size = entity_properties[self.entity_type.clone() as usize].size;
        Rect::new(self.position, self.position + Vec2i::both(size))
    }
}

#[derive(Clone, Debug)]
pub struct SimulatedEntityAction {
    pub entity_id: i32,
    pub action_type: SimulatedEntityActionType,
}

#[derive(Clone, Debug)]
pub enum SimulatedEntityActionType {
    None,
    Attack {
        target: i32,
    },
    MoveEntity {
        direction: Vec2i,
    },
    AutoAttack,
}

impl SimulatedEntityActionType {
    fn priority(&self) -> usize {
        match self {
            Self::Attack { .. } => 0,
            Self::MoveEntity { .. } => 1,
            _ => 2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EntitySimulator {
    shift: Vec2i,
    map_size: usize,
    entities: Vec<SimulatedEntity>,
    tiles: Vec<Option<i32>>,
    actions: Vec<SimulatedEntityAction>,
    players: Vec<SimulatedPlayer>,
}

impl EntitySimulator {
    pub fn new(shift: Vec2i, map_size: usize, world: &World) -> Self {
        let mut tiles: Vec<Option<i32>> = std::iter::repeat(None).take(map_size * map_size).collect();
        let mut entities: Vec<SimulatedEntity> = Vec::new();
        world.visit_map_square(shift, map_size as i32, |position, tile, _| {
            if let Tile::Entity(entity_id) = tile {
                let entity = world.get_entity(entity_id);
                tiles[position_to_index(position - shift, map_size)] = Some(entity.id);
                if entities.iter().any(|v| v.id == entity_id) {
                    return;
                }
                entities.push(SimulatedEntity {
                    id: entity.id,
                    entity_type: entity.entity_type.clone(),
                    position: entity.position(),
                    player_id: entity.player_id,
                    health: entity.health,
                    active: entity.active,
                    available: true,
                });
            }
        });
        entities.sort_by_key(|v| v.id);
        Self {
            map_size,
            shift,
            entities,
            tiles,
            players: world.players().iter()
                .map(|player| SimulatedPlayer {
                    id: player.id,
                    score: player.score,
                    damage_done: 0,
                    damage_received: 0,
                })
                .collect(),
            actions: Vec::new(),
        }
    }

    pub fn shift(&self) -> Vec2i {
        self.shift
    }

    pub fn map_size(&self) -> usize {
        self.map_size
    }

    pub fn players(&self) -> &Vec<SimulatedPlayer> {
        &self.players
    }

    pub fn entities(&self) -> &Vec<SimulatedEntity> {
        &self.entities
    }

    pub fn tiles(&self) -> &Vec<Option<i32>> {
        &self.tiles
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(self.shift, self.shift + Vec2i::both(self.map_size as i32))
    }

    pub fn get_entity(&self, entity_id: i32) -> &SimulatedEntity {
        self.entities.iter().find(|v| v.id == entity_id).unwrap()
    }

    pub fn add_action(&mut self, action: SimulatedEntityAction) {
        self.actions.push(action);
    }

    pub fn simulate<R: Rng>(&mut self, entity_properties: &Vec<EntityProperties>, rng: &mut R) {
        for entity in self.entities.iter_mut() {
            entity.available = true;
        }
        for action_index in 0..self.actions.len() {
            if matches!(self.actions[action_index].action_type, SimulatedEntityActionType::AutoAttack) {
                let entity_index = self.get_entity_index(self.actions[action_index].entity_id);
                if self.entities[entity_index].available && self.entities[entity_index].active {
                    self.actions[action_index].action_type = self.get_auto_attack_action(entity_index, entity_properties);
                } else {
                    self.actions[action_index].action_type = SimulatedEntityActionType::None;
                }
            }
        }
        self.actions.shuffle(rng);
        for action_index in 0..self.actions.len() {
            if let SimulatedEntityActionType::Attack { target } = self.actions[action_index].action_type.clone() {
                let entity_index = self.get_entity_index(self.actions[action_index].entity_id);
                if !self.entities[entity_index].available || !self.entities[entity_index].active {
                    continue;
                }
                if let Some((target_index, _)) = self.entities.iter().find_position(|v| v.id == target) {
                    self.attack(entity_index, target_index, entity_properties);
                    self.entities[entity_index].available = false;
                }
            }
        }
        for action_index in 0..self.actions.len() {
            if let SimulatedEntityActionType::MoveEntity { direction } = self.actions[action_index].action_type.clone() {
                let entity_index = self.get_entity_index(self.actions[action_index].entity_id);
                if !self.entities[entity_index].available || !self.entities[entity_index].active
                    || self.entities[entity_index].health <= 0 {
                    continue;
                }
                self.move_entity(entity_index, direction, entity_properties);
                self.entities[entity_index].available = false;
            }
        }
        self.actions.clear();
        let bounds = self.bounds();
        for i in 0..self.entities.len() {
            if self.entities[i].health <= 0 {
                let size = entity_properties[self.entities[i].entity_type.clone() as usize].size;
                visit_square_with_bounds(self.entities[i].position, size, &bounds, |position| {
                    self.tiles[position_to_index(position - self.shift, self.map_size)] = None;
                });
            }
        }
        self.entities.retain(|v| v.health > 0 && bounds.overlaps(&v.bounds(entity_properties)));
    }

    fn get_entity_index(&self, entity_id: i32) -> usize {
        self.entities.iter().find_position(|v| v.id == entity_id).unwrap().0
    }

    fn attack(&mut self, entity_index: usize, target_index: usize, entity_properties: &Vec<EntityProperties>) {
        if self.entities[target_index].health <= 0 || !self.bounds().contains(self.entities[target_index].position) {
            return;
        }
        let properties = &entity_properties[self.entities[entity_index].entity_type.clone() as usize];
        let target_properties = &entity_properties[self.entities[target_index].entity_type.clone() as usize];
        if let Some(attack) = properties.attack.as_ref() {
            let entity_bounds = self.entities[entity_index].bounds(entity_properties);
            let target_bounds = self.entities[target_index].bounds(entity_properties);
            if entity_bounds.distance(&target_bounds) > attack.attack_range {
                return;
            }
            let health = self.entities[target_index].health;
            self.entities[target_index].health -= attack.damage;
            if let Some(target_player_id) = self.entities[target_index].player_id {
                let damage = health - self.entities[target_index].health;
                self.players.iter_mut().find(|v| v.id == target_player_id).unwrap().damage_received += damage;
                if let Some(entity_player_id) = self.entities[entity_index].player_id {
                    self.players.iter_mut().find(|v| v.id == entity_player_id).unwrap().damage_done += damage;
                    if self.entities[target_index].health <= 0 {
                        self.players.iter_mut().find(|v| v.id == entity_player_id).unwrap().score += target_properties.destroy_score;
                    }
                }
            }
        }
    }

    fn move_entity(&mut self, entity_index: usize, direction: Vec2i, entity_properties: &Vec<EntityProperties>) {
        assert!(direction.abs().sum() <= 1, "{:?}", direction);
        let properties = &entity_properties[self.entities[entity_index].entity_type.clone() as usize];
        if !properties.can_move {
            return;
        }
        let position = self.entities[entity_index].position;
        let target_position = position + direction;
        let target_position_index = position_to_index(target_position - self.shift, self.map_size);
        if self.bounds().contains(target_position) {
            if self.tiles[target_position_index].is_some() {
                return;
            }
            self.tiles[target_position_index] = Some(self.entities[entity_index].id);
        }
        self.tiles[position_to_index(position - self.shift, self.map_size)] = None;
        self.entities[entity_index].position = target_position;
    }

    fn get_auto_attack_action(&mut self, entity_index: usize, entity_properties: &Vec<EntityProperties>) -> SimulatedEntityActionType {
        let entity = &self.entities[entity_index];
        let properties = &entity_properties[entity.entity_type.clone() as usize];
        let entity_bounds = entity.bounds(entity_properties);
        if let Some(attack) = properties.attack.as_ref() {
            let target = self.entities.iter()
                .filter(|other| {
                    other.id != entity.id && other.player_id.is_some() && other.player_id != entity.player_id && other.health > 0
                })
                .filter_map(|target| {
                    let distance = target.bounds(entity_properties).distance(&entity_bounds);
                    if distance > properties.sight_range {
                        return None;
                    }
                    Some((distance, target))
                })
                .min_by_key(|(distance, _)| *distance);
            if let Some((distance, target)) = target {
                if distance <= attack.attack_range {
                    return SimulatedEntityActionType::Attack { target: target.id };
                } else if properties.can_move {
                    if let Some(next_position) = self.find_shortest_path_next_position(entity.position, target, attack.attack_range, entity_properties) {
                        let direction = next_position - entity.position;
                        assert_eq!(direction.abs().sum(), 1, "{:?}", direction);
                        return SimulatedEntityActionType::MoveEntity { direction };
                    }
                }
            }
        }
        SimulatedEntityActionType::None
    }

    fn find_shortest_path_next_position(&self, src: Vec2i, target: &SimulatedEntity, range: i32, entity_properties: &Vec<EntityProperties>) -> Option<Vec2i> {
        let bounds = self.bounds();
        let target_bounds = target.bounds(entity_properties);
        let size = self.map_size;

        let mut open: Vec<bool> = std::iter::repeat(true)
            .take(size * size)
            .collect();
        let mut costs: Vec<i32> = std::iter::repeat(std::i32::MAX)
            .take(size * size)
            .collect();
        let mut backtrack: Vec<usize> = (0..(size * size)).into_iter().collect();
        let mut discovered = BinaryHeap::new();

        let src_index = position_to_index(src - self.shift, size);

        costs[src_index] = 0;
        discovered.reserve(2 * size);
        discovered.push((-target_bounds.distance_to_position(src), src_index));

        const EDGES: &[Vec2i] = &[
            Vec2i::only_x(1),
            Vec2i::only_x(-1),
            Vec2i::only_y(1),
            Vec2i::only_y(-1),
        ];

        let mut nearest_position_index = None;
        let mut min_distance = std::i32::MAX;

        while let Some((_, node_index)) = discovered.pop() {
            let node_position = index_to_position(node_index, size);
            let distance = target_bounds.distance_to_position(node_position + self.shift);
            if min_distance > distance {
                min_distance = distance;
                nearest_position_index = Some(node_index);
                if distance <= range {
                    break;
                }
            }
            open[node_index] = true;
            for &shift in EDGES.iter() {
                let neighbour_position = node_position + shift;
                if !bounds.contains(neighbour_position + self.shift) {
                    continue;
                }
                let neighbour_index = position_to_index(neighbour_position, size);
                if self.tiles[neighbour_index].is_some() {
                    continue;
                }
                let new_cost = costs[node_index] + 1;
                if costs[neighbour_index] <= new_cost {
                    continue;
                }
                costs[neighbour_index] = new_cost;
                backtrack[neighbour_index] = node_index;
                if !open[neighbour_index] {
                    continue;
                }
                open[neighbour_index] = false;
                let new_score = new_cost + target_bounds.distance_to_position(neighbour_position + self.shift);
                discovered.push((-new_score, neighbour_index));
            }
        }

        if let Some(dst) = nearest_position_index {
            let mut first_position_index = None;
            let success = visit_reversed_shortest_path(src_index, dst, &backtrack, |index| {
                first_position_index = Some(index);
            });
            if success {
                return first_position_index.map(|v| index_to_position(v, size) + self.shift);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use model::{Entity, Player, PlayerView, Vec2I32};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    use crate::my_strategy::examples;

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
                    entity_type: EntityType::BuilderUnit,
                    position: Vec2I32 { x: 20, y: 20 },
                    health: entity_properties[&EntityType::BuilderUnit].max_health,
                    active: true,
                },
                Entity {
                    id: 2,
                    player_id: Some(1),
                    entity_type: EntityType::RangedUnit,
                    position: Vec2I32 { x: 30, y: 30 },
                    health: entity_properties[&EntityType::RangedUnit].max_health,
                    active: true,
                },
                Entity {
                    id: 3,
                    player_id: Some(2),
                    entity_type: EntityType::RangedUnit,
                    position: Vec2I32 { x: 35, y: 30 },
                    health: entity_properties[&EntityType::RangedUnit].max_health,
                    active: true,
                },
                Entity {
                    id: 4,
                    player_id: Some(1),
                    entity_type: EntityType::MeleeUnit,
                    position: Vec2I32 { x: 30, y: 35 },
                    health: entity_properties[&EntityType::MeleeUnit].max_health,
                    active: true,
                },
                Entity {
                    id: 5,
                    player_id: Some(2),
                    entity_type: EntityType::MeleeUnit,
                    position: Vec2I32 { x: 35, y: 35 },
                    health: entity_properties[&EntityType::MeleeUnit].max_health,
                    active: true,
                },
            ],
            entity_properties,
        }
    }

    fn new_world() -> World {
        let player_view = new_player_view();
        let mut world = World::new(&player_view);
        world.update(&player_view);
        world
    }

    #[test]
    fn simulate() {
        let world = new_world();
        let mut simulator = EntitySimulator::new(Vec2i::new(20, 20), 20, &world);
        let mut rng = StdRng::seed_from_u64(42);
        simulator.simulate(world.entity_properties(), &mut rng);
        assert_eq!(simulator.entities().len(), 5);
        assert_eq!(simulator.players().len(), 2);
        assert_eq!(simulator.players()[0].id, 1);
        assert_eq!(simulator.players()[1].id, 2);
        assert_eq!(simulator.players()[0].score, 0);
        assert_eq!(simulator.players()[1].score, 0);
        assert_eq!(simulator.players()[0].damage_received, 0);
        assert_eq!(simulator.players()[1].damage_received, 0);
        assert_eq!(simulator.players()[0].damage_done, 0);
        assert_eq!(simulator.players()[1].damage_done, 0);
    }

    #[test]
    fn simulate_move_entity() {
        let world = new_world();
        let mut simulator = EntitySimulator::new(Vec2i::new(20, 20), 20, &world);
        let mut rng = StdRng::seed_from_u64(42);
        simulator.add_action(SimulatedEntityAction {
            entity_id: 1,
            action_type: SimulatedEntityActionType::MoveEntity { direction: Vec2i::new(1, 0) },
        });
        simulator.simulate(world.entity_properties(), &mut rng);
        assert_eq!(simulator.entities()[0].id, 1);
        assert_eq!(simulator.entities()[0].position, Vec2i::new(21, 20));
    }

    #[test]
    fn simulate_move_entity_outside() {
        let world = new_world();
        let mut simulator = EntitySimulator::new(Vec2i::new(20, 20), 20, &world);
        let mut rng = StdRng::seed_from_u64(42);
        simulator.add_action(SimulatedEntityAction {
            entity_id: 1,
            action_type: SimulatedEntityActionType::MoveEntity { direction: Vec2i::new(-1, 0) },
        });
        simulator.simulate(world.entity_properties(), &mut rng);
        assert_eq!(simulator.entities().len(), 4);
        assert!(!simulator.entities().iter().any(|v| v.id == 1));
    }

    #[test]
    fn simulate_attack_in_range() {
        let world = new_world();
        let mut simulator = EntitySimulator::new(Vec2i::new(20, 20), 20, &world);
        let mut rng = StdRng::seed_from_u64(42);
        simulator.add_action(SimulatedEntityAction {
            entity_id: 2,
            action_type: SimulatedEntityActionType::Attack { target: 3 },
        });
        assert_eq!(simulator.entities()[2].id, 3);
        assert_eq!(simulator.entities()[2].health, 10);
        simulator.simulate(world.entity_properties(), &mut rng);
        assert_eq!(simulator.entities()[2].id, 3);
        assert_eq!(simulator.entities()[2].health, 5);
    }

    #[test]
    fn simulate_attack_out_of_range() {
        let world = new_world();
        let mut simulator = EntitySimulator::new(Vec2i::new(20, 20), 20, &world);
        let mut rng = StdRng::seed_from_u64(42);
        simulator.add_action(SimulatedEntityAction {
            entity_id: 4,
            action_type: SimulatedEntityActionType::Attack { target: 5 },
        });
        assert_eq!(simulator.entities()[4].id, 5);
        assert_eq!(simulator.entities()[4].health, 50);
        simulator.simulate(world.entity_properties(), &mut rng);
        assert_eq!(simulator.entities()[4].id, 5);
        assert_eq!(simulator.entities()[4].health, 50);
    }

    #[test]
    fn simulate_auto_attack_in_range() {
        let world = new_world();
        let mut simulator = EntitySimulator::new(Vec2i::new(20, 20), 20, &world);
        let mut rng = StdRng::seed_from_u64(42);
        simulator.add_action(SimulatedEntityAction {
            entity_id: 3,
            action_type: SimulatedEntityActionType::AutoAttack,
        });
        assert_eq!(simulator.entities()[1].id, 2);
        assert_eq!(simulator.entities()[1].health, 10);
        simulator.simulate(world.entity_properties(), &mut rng);
        assert_eq!(simulator.entities()[1].id, 2);
        assert_eq!(simulator.entities()[1].health, 5);
    }

    #[test]
    fn simulate_auto_attack_out_of_range() {
        let world = new_world();
        let mut simulator = EntitySimulator::new(Vec2i::new(20, 20), 20, &world);
        let mut rng = StdRng::seed_from_u64(42);
        simulator.add_action(SimulatedEntityAction {
            entity_id: 5,
            action_type: SimulatedEntityActionType::AutoAttack,
        });
        assert_eq!(simulator.entities()[4].id, 5);
        assert_eq!(simulator.entities()[4].position, Vec2i::new(35, 35));
        simulator.simulate(world.entity_properties(), &mut rng);
        assert_eq!(simulator.entities()[4].id, 5);
        assert_eq!(simulator.entities()[4].position, Vec2i::new(34, 35));
    }

    #[test]
    fn simulate_all_auto_attack() {
        let world = new_world();
        let mut simulator = EntitySimulator::new(Vec2i::new(20, 20), 20, &world);
        let mut rng = StdRng::seed_from_u64(42);
        while simulator.entities().len() > 1 {
            for i in 0..simulator.entities().len() {
                simulator.add_action(SimulatedEntityAction {
                    entity_id: simulator.entities()[i].id,
                    action_type: SimulatedEntityActionType::AutoAttack,
                });
            }
            simulator.simulate(world.entity_properties(), &mut rng);
        }
        assert_eq!(simulator.players()[0].id, 1);
        assert_eq!(simulator.players()[1].id, 2);
        assert_eq!(simulator.players()[0].score, 500);
        assert_eq!(simulator.players()[1].score, 500);
        assert_eq!(simulator.players()[0].damage_received, 60);
        assert_eq!(simulator.players()[1].damage_received, 60);
        assert_eq!(simulator.players()[0].damage_done, 60);
        assert_eq!(simulator.players()[1].damage_done, 60);
    }
}
