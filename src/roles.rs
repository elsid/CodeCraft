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

use crate::my_strategy::{
    Group,
    Positionable,
    Vec2i,
    World,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Role {
    None,
    Harvester {
        position: Vec2i,
    },
    UnitBuilder,
    BuildingBuilder {
        position: Vec2i,
        entity_type: EntityType,
    },
    BuildingRepairer {
        building_id: i32,
    },
    GroupMember {
        group_id: usize,
    },
    GroupSupplier {
        group_id: usize,
    },
}

impl Role {
    pub fn get_action(&self, entity: &Entity, world: &World, groups: &HashMap<usize, Group>) -> EntityAction {
        match self {
            Role::Harvester { position } => harvest_resources(entity, world, *position),
            Role::UnitBuilder => build_unit(entity, world),
            Role::BuildingBuilder { position, entity_type } => build_building(entity, world, *position, entity_type),
            Role::BuildingRepairer { building_id: base_id } => repair_building(entity, world, *base_id),
            Role::GroupMember { group_id } => assist_group(entity, world, &groups[group_id]),
            Role::GroupSupplier { .. } => build_unit(entity, world),
            Role::None => get_default_action(entity, world),
        }
    }

    pub fn is_temporary(&self) -> bool {
        match self {
            Role::UnitBuilder => true,
            _ => false,
        }
    }
}

fn harvest_resources(builder: &Entity, world: &World, position: Vec2i) -> EntityAction {
    let builder_properties = world.get_entity_properties(&EntityType::BuilderUnit);
    let builder_attack_properties = builder_properties.attack.as_ref().unwrap();
    EntityAction {
        attack_action: Some(AttackAction {
            target: None,
            auto_attack: Some(AutoAttack {
                pathfind_range: builder_attack_properties.attack_range,
                valid_targets: vec![EntityType::Resource, EntityType::BuilderUnit],
            }),
        }),
        build_action: None,
        repair_action: None,
        move_action: if position != builder.position() {
            Some(MoveAction {
                target: position.as_model(),
                break_through: true,
                find_closest_position: true,
            })
        } else {
            None
        },
    }
}

fn build_unit(base: &Entity, world: &World) -> EntityAction {
    let properties = world.get_entity_properties(&base.entity_type);
    if let Some(build_properties) = properties.build.as_ref() {
        if let Some(position) = world.find_free_tile_nearby(base.position(), properties.size) {
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
    get_idle_action()
}

fn build_building(builder: &Entity, world: &World, position: Vec2i, entity_type: &EntityType) -> EntityAction {
    let target = world.find_nearest_free_tile_nearby_for_unit(
        position,
        world.get_entity_properties(entity_type).size,
        builder.id,
    );
    target
        .map(|v| EntityAction {
            attack_action: None,
            build_action: Some(BuildAction {
                position: position.as_model(),
                entity_type: entity_type.clone(),
            }),
            repair_action: None,
            move_action: Some(MoveAction {
                target: v.as_model(),
                find_closest_position: false,
                break_through: false,
            }),
        })
        .unwrap_or_else(get_idle_action)
}

fn repair_building(builder: &Entity, world: &World, base_id: i32) -> EntityAction {
    let base = world.get_entity(base_id);
    let target = world.find_nearest_free_tile_nearby_for_unit(
        base.position(),
        world.get_entity_properties(&base.entity_type).size,
        builder.id,
    );
    target
        .map(|v| EntityAction {
            attack_action: None,
            build_action: None,
            repair_action: Some(RepairAction { target: base_id }),
            move_action: Some(MoveAction {
                target: v.as_model(),
                find_closest_position: true,
                break_through: false,
            }),
        })
        .unwrap_or_else(get_idle_action)
}

fn assist_group(unit: &Entity, world: &World, group: &Group) -> EntityAction {
    let properties = world.get_entity_properties(&unit.entity_type);
    let unit_center = unit.center(properties.size);
    let repair_action = properties.repair.as_ref()
        .and_then(|_| {
            group.units()
                .filter_map(|unit_id| {
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
    let attack_action = properties.attack.as_ref()
        .and_then(|_| {
            world.opponent_entities()
                .find(|v| {
                    v.center(world.get_entity_properties(&v.entity_type).size).distance(unit_center)
                        <= properties.sight_range
                })
                .map(|_| AttackAction {
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
                })
        });
    if let Some(attack) = attack_action {
        return EntityAction {
            build_action: None,
            repair_action: None,
            move_action: attack.target.map(|v| MoveAction {
                target: world.get_entity(v).position.clone(),
                find_closest_position: true,
                break_through: true,
            }),
            attack_action: Some(attack),
        };
    }
    EntityAction {
        attack_action: None,
        build_action: None,
        repair_action: None,
        move_action: group.target()
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
