use model::EntityType;

pub fn is_entity_type_base(entity_type: &EntityType) -> bool {
    match entity_type {
        EntityType::BuilderBase => true,
        EntityType::MeleeBase => true,
        EntityType::RangedBase => true,
        _ => false,
    }
}

pub fn is_entity_type_unit(entity_type: &EntityType) -> bool {
    match entity_type {
        EntityType::BuilderUnit => true,
        EntityType::MeleeUnit => true,
        EntityType::RangedUnit => true,
        _ => false,
    }
}
