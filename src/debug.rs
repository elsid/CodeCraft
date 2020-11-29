use model::{
    Color,
    ColoredVertex,
};

use crate::my_strategy::Vec2f;

pub fn add_world_square(position: Vec2f, size: f64, color: Color, vertices: &mut Vec<ColoredVertex>) {
    const SHIFTS: &[Vec2f] = &[
        Vec2f::zero(),
        Vec2f::only_x(1.0),
        Vec2f::both(1.0),
        Vec2f::zero(),
        Vec2f::only_y(1.0),
        Vec2f::both(1.0),
    ];
    for shift in SHIFTS.iter() {
        vertices.push(ColoredVertex {
            world_pos: Some((position + *shift * size).as_model()),
            screen_offset: Vec2f::zero().as_model(),
            color: color.clone(),
        });
    }
}

pub fn add_world_rectangle(min: Vec2f, max: Vec2f, color: Color, vertices: &mut Vec<ColoredVertex>) {
    let positions = &[
        min, Vec2f::new(min.x(), max.y()), max,
        min, Vec2f::new(max.x(), min.y()), max
    ];
    for position in positions.iter() {
        vertices.push(ColoredVertex {
            world_pos: Some(position.as_model()),
            screen_offset: Vec2f::zero().as_model(),
            color: color.clone(),
        });
    }
}

pub fn add_world_line(begin: Vec2f, end: Vec2f, color: Color, vertices: &mut Vec<ColoredVertex>) {
    vertices.push(ColoredVertex {
        world_pos: Some(begin.as_model()),
        screen_offset: Vec2f::zero().as_model(),
        color: color.clone(),
    });
    vertices.push(ColoredVertex {
        world_pos: Some(end.as_model()),
        screen_offset: Vec2f::zero().as_model(),
        color: color.clone(),
    });
}
