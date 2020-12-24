use std::collections::HashMap;

use model::{
    AttackAction,
    AutoAttack,
    BuildAction,
    Entity,
    EntityAction,
    EntityType,
    MoveAction,
    RepairAction,
};

use crate::my_strategy::{EntityPlanner, Group, Positionable, Rect, SimulatedEntityActionType, SizedRange, Vec2i, World};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Role {
    None,
    Harvester {
        resource_id: i32,
    },
    UnitBuilder,
    BuildingBuilder {
        position: Vec2i,
        entity_type: EntityType,
    },
    BuildingRepairer {
        building_id: i32,
        need_resources: bool,
    },
    GroupMember {
        group_id: u32,
    },
    GroupSupplier {
        group_id: u32,
    },
    Cleaner {
        resource_id: i32,
    },
    Fighter,
    Scout,
}

impl Role {
    pub fn get_action(&self, entity: &Entity, world: &World, groups: &Vec<Group>, entity_targets: &HashMap<i32, Vec2i>, entity_planners: &HashMap<i32, EntityPlanner>) -> EntityAction {
        match self {
            Role::Harvester { resource_id } => harvest_resource(entity, world, *resource_id),
            Role::UnitBuilder => build_unit(entity, world),
            Role::BuildingBuilder { position, entity_type } => build_building(entity, world, *position, entity_type),
            Role::BuildingRepairer { building_id: base_id, need_resources } => repair_building(entity, world, *base_id, *need_resources),
            Role::GroupMember { group_id } => assist_group(entity, world, groups.iter().find(|v| v.id() == *group_id).unwrap(), entity_targets, entity_planners),
            Role::GroupSupplier { .. } => build_unit(entity, world),
            Role::None => get_default_action(entity, world),
            Role::Cleaner { resource_id } => harvest_resource(entity, world, *resource_id),
            Role::Fighter | Role::Scout => fight(entity, world, None, entity_targets, entity_planners),
        }
    }

    pub fn is_temporary(&self) -> bool {
        match self {
            Role::UnitBuilder => true,
            _ => false,
        }
    }
}

fn harvest_resource(entity: &Entity, world: &World, resource_id: i32) -> EntityAction {
    let builder_properties = world.get_entity_properties(&EntityType::BuilderUnit);
    EntityAction {
        attack_action: if world.is_attacked_by_opponents(entity.position()) {
            let builder_attack_properties = builder_properties.attack.as_ref().unwrap();
            Some(AttackAction {
                target: None,
                auto_attack: Some(AutoAttack {
                    pathfind_range: builder_attack_properties.attack_range,
                    valid_targets: vec![EntityType::BuilderUnit],
                }),
            })
        } else {
            Some(AttackAction {
                target: Some(resource_id),
                auto_attack: Some(AutoAttack {
                    pathfind_range: 2 * world.map_size(),
                    valid_targets: vec![EntityType::BuilderUnit, EntityType::Resource],
                }),
            })
        },
        build_action: None,
        repair_action: None,
        move_action: Some(MoveAction {
            target: world.get_entity(resource_id).position.clone(),
            break_through: true,
            find_closest_position: true,
        }),
    }
}

fn build_unit(base: &Entity, world: &World) -> EntityAction {
    let properties = world.get_entity_properties(&base.entity_type);
    if let Some(build_properties) = properties.build.as_ref() {
        if matches!(build_properties.options[0], EntityType::BuilderUnit) && !world.harvest_positions().is_empty() {
            let mut min_distance = std::i32::MAX;
            let mut build_position = None;
            world.visit_free_tiles_nearby(base.position(), properties.size, |position| {
                let distance = world.harvest_positions().iter()
                    .map(|v| v.distance(position))
                    .min()
                    .unwrap();
                if min_distance > distance {
                    build_position = Some(position);
                    min_distance = distance;
                }
            });
            if let Some(position) = build_position {
                return EntityAction {
                    attack_action: None,
                    build_action: Some(BuildAction {
                        position: position.as_model(),
                        entity_type: build_properties.options[0].clone(),
                    }),
                    repair_action: None,
                    move_action: None,
                };
            }
        } else {
            let mut max_distance = std::i32::MIN;
            let mut build_position = None;
            world.visit_free_tiles_nearby(base.position(), properties.size, |position| {
                let distance = world.start_position().distance(position);
                if max_distance < distance {
                    build_position = Some(position);
                    max_distance = distance;
                }
            });
            if let Some(position) = build_position {
                return EntityAction {
                    attack_action: None,
                    build_action: Some(BuildAction {
                        position: position.as_model(),
                        entity_type: build_properties.options[0].clone(),
                    }),
                    repair_action: None,
                    move_action: None,
                };
            }
        }
    }
    get_idle_action()
}

fn build_building(builder: &Entity, world: &World, position: Vec2i, entity_type: &EntityType) -> EntityAction {
    let size = world.get_entity_properties(entity_type).size;
    get_target_position_nearby(builder.position(), position, size, world)
        .map(|target| EntityAction {
            attack_action: None,
            build_action: if target == builder.position() {
                Some(BuildAction {
                    position: position.as_model(),
                    entity_type: entity_type.clone(),
                })
            } else {
                None
            },
            repair_action: None,
            move_action: Some(MoveAction {
                target: target.as_model(),
                find_closest_position: false,
                break_through: false,
            }),
        })
        .unwrap_or_else(get_idle_action)
}

fn repair_building(builder: &Entity, world: &World, base_id: i32, need_resources: bool) -> EntityAction {
    if need_resources &&
        !world.try_allocate_resource(world.get_entity_properties(&EntityType::BuilderUnit).repair.as_ref().unwrap().power) {
        return get_idle_action();
    }
    let base = world.get_entity(base_id);
    let size = world.get_entity_properties(&base.entity_type).size;
    get_target_position_nearby(builder.position(), base.position(), size, world)
        .map(|target| {
            EntityAction {
                attack_action: None,
                build_action: None,
                repair_action: if target == builder.position() {
                    Some(RepairAction { target: base_id })
                } else {
                    None
                },
                move_action: Some(MoveAction {
                    target: target.as_model(),
                    find_closest_position: true,
                    break_through: false,
                }),
            }
        })
        .unwrap_or_else(get_idle_action)
}

fn get_target_position_nearby(position: Vec2i, target: Vec2i, size: i32, world: &World) -> Option<Vec2i> {
    let bounds = Rect::new(target, target + Vec2i::both(size));
    if bounds.distance_to_position(position) == 1 {
        return Some(position);
    }
    world.find_shortest_path_next_position(position, &SizedRange::new(target, size, 1), false)
}

fn assist_group(unit: &Entity, world: &World, group: &Group, entity_targets: &HashMap<i32, Vec2i>, entity_planners: &HashMap<i32, EntityPlanner>) -> EntityAction {
    let properties = world.get_entity_properties(&unit.entity_type);
    let unit_center = unit.center(properties.size);
    let repair_action = properties.repair.as_ref()
        .and_then(|_| {
            group.units().iter()
                .filter_map(|(unit_id, _)| {
                    if *unit_id != unit.id {
                        return None;
                    }
                    let other = world.get_entity(*unit_id);
                    let other_properties = world.get_entity_properties(&other.entity_type);
                    let damage = other_properties.max_health - other.health;
                    if damage == 0 {
                        return None;
                    }
                    let distance = other.center(other_properties.size).distance(unit_center);
                    if distance > properties.sight_range {
                        return None;
                    }
                    Some((distance, damage, unit_id))
                })
                .min()
                .map(|(_, _, unit_id)| RepairAction { target: *unit_id })
        });
    if let Some(repair) = repair_action {
        return EntityAction {
            attack_action: None,
            build_action: None,
            move_action: Some(MoveAction {
                target: world.get_entity(repair.target).position.clone(),
                find_closest_position: true,
                break_through: false,
            }),
            repair_action: Some(repair),
        };
    }
    fight(unit, world, group.target(), entity_targets, entity_planners)
}

fn fight(entity: &Entity, world: &World, default_target: Option<Vec2i>, entity_targets: &HashMap<i32, Vec2i>, entity_planners: &HashMap<i32, EntityPlanner>) -> EntityAction {
    if let Some(action) = get_action_by_plan(entity, world, entity_planners) {
        return action;
    }
    EntityAction {
        attack_action: Some(AttackAction {
            target: None,
            auto_attack: Some(AutoAttack {
                pathfind_range: world.get_entity_properties(&entity.entity_type).sight_range,
                valid_targets: vec![],
            }),
        }),
        build_action: None,
        repair_action: None,
        move_action: entity_targets.get(&entity.id).cloned().or(default_target)
            .map(|position| MoveAction {
                target: position.as_model(),
                find_closest_position: true,
                break_through: true,
            }),
    }
}

fn get_default_action(entity: &Entity, world: &World) -> EntityAction {
    let properties = world.get_entity_properties(&entity.entity_type);
    if properties.attack.is_some() {
        return EntityAction {
            attack_action: Some(AttackAction {
                target: None,
                auto_attack: Some(AutoAttack {
                    pathfind_range: properties.sight_range,
                    valid_targets: vec![
                        EntityType::BuilderUnit,
                        EntityType::MeleeUnit,
                        EntityType::RangedUnit,
                        EntityType::Turret,
                        EntityType::House,
                        EntityType::BuilderBase,
                        EntityType::MeleeBase,
                        EntityType::RangedBase,
                        EntityType::Wall,
                    ],
                }),
            }),
            build_action: None,
            repair_action: None,
            move_action: None,
        };
    }
    get_idle_action()
}

fn get_idle_action() -> EntityAction {
    EntityAction {
        attack_action: None,
        build_action: None,
        repair_action: None,
        move_action: None,
    }
}

fn get_action_by_plan(entity: &Entity, world: &World, entity_planners: &HashMap<i32, EntityPlanner>) -> Option<EntityAction> {
    if let Some(planner) = entity_planners.get(&entity.id) {
        let plan = planner.plan();
        if !plan.transitions.is_empty() {
            return Some(make_action(entity, &plan.transitions[0], world));
        }
    }
    None
}

fn make_action(entity: &Entity, action_type: &SimulatedEntityActionType, world: &World) -> EntityAction {
    match action_type {
        SimulatedEntityActionType::None => {
            EntityAction {
                attack_action: None,
                build_action: None,
                repair_action: None,
                move_action: None,
            }
        }
        SimulatedEntityActionType::Attack { target } => {
            EntityAction {
                attack_action: Some(AttackAction {
                    target: Some(*target),
                    auto_attack: None,
                }),
                build_action: None,
                repair_action: None,
                move_action: None,
            }
        }
        SimulatedEntityActionType::MoveEntity { direction } => {
            EntityAction {
                attack_action: None,
                build_action: None,
                repair_action: None,
                move_action: Some(MoveAction {
                    target: (entity.position() + *direction).as_model(),
                    find_closest_position: false,
                    break_through: false,
                }),
            }
        }
        SimulatedEntityActionType::AutoAttack | SimulatedEntityActionType::AttackInRange => {
            EntityAction {
                attack_action: Some(AttackAction {
                    target: None,
                    auto_attack: Some(AutoAttack {
                        pathfind_range: world.get_entity_properties(&entity.entity_type).sight_range,
                        valid_targets: vec![],
                    }),
                }),
                build_action: None,
                repair_action: None,
                move_action: None,
            }
        }
    }
}
