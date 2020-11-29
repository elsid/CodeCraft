use model::Entity;

use crate::my_strategy::{
    is_entity_type_base,
    is_entity_type_unit,
    Positionable,
    Vec2i,
};

impl Positionable for Entity {
    fn position(&self) -> Vec2i {
        Vec2i::from(self.position.clone())
    }
}

pub fn is_entity_base(entity: &Entity) -> bool {
    is_entity_type_base(&entity.entity_type)
}

pub fn is_entity_unit(entity: &Entity) -> bool {
    is_entity_type_unit(&entity.entity_type)
}
