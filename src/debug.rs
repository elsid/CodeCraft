use model::{
    Color,
    ColoredVertex,
    DebugCommand,
    DebugData,
    DebugState,
    PrimitiveType,
};

use crate::DebugInterface;
use crate::my_strategy::Vec2f;

pub struct Debug<'a> {
    state: &'a DebugState,
    line_vertices: Vec<ColoredVertex>,
    triangle_vertices: Vec<ColoredVertex>,
    world_texts: Vec<DebugData>,
    static_texts: Vec<DebugData>,
    next_screen_offset: f32,
}

impl<'a> Debug<'a> {
    pub fn new(state: &'a DebugState) -> Self {
        Self {
            state,
            line_vertices: Vec::new(),
            triangle_vertices: Vec::new(),
            world_texts: Vec::new(),
            static_texts: Vec::new(),
            next_screen_offset: 32.0,
        }
    }

    pub fn add_static_text(&mut self, text: String) {
        self.static_texts.push(DebugData::PlacedText {
            text,
            vertex: ColoredVertex {
                world_pos: None,
                screen_offset: Vec2f::new(32.0, self.state.window_size.y as f32 - self.next_screen_offset).as_model(),
                color: Color { a: 1.0, r: 1.0, g: 1.0, b: 1.0 },
            },
            alignment: 0.0,
            size: 26.0,
        });
        self.next_screen_offset += 32.0;
    }

    pub fn add_world_text(&mut self, text: String, world_position: Vec2f, screen_offset: Vec2f, color: Color) {
        self.world_texts.push(DebugData::PlacedText {
            text,
            vertex: ColoredVertex {
                world_pos: Some(world_position.as_model()),
                screen_offset: screen_offset.as_model(),
                color,
            },
            alignment: 0.5,
            size: 26.0,
        });
    }

    pub fn add_world_square(&mut self, position: Vec2f, size: f32, color: Color) {
        const SHIFTS: &[Vec2f] = &[
            Vec2f::zero(),
            Vec2f::only_x(1.0),
            Vec2f::both(1.0),
            Vec2f::zero(),
            Vec2f::only_y(1.0),
            Vec2f::both(1.0),
        ];
        for shift in SHIFTS.iter() {
            self.triangle_vertices.push(ColoredVertex {
                world_pos: Some((position + *shift * size).as_model()),
                screen_offset: Vec2f::zero().as_model(),
                color: color.clone(),
            });
        }
    }

    pub fn add_world_rectangle(&mut self, min: Vec2f, max: Vec2f, color: Color) {
        let positions = &[
            min, Vec2f::new(min.x(), max.y()), max,
            min, Vec2f::new(max.x(), min.y()), max
        ];
        for position in positions.iter() {
            self.triangle_vertices.push(ColoredVertex {
                world_pos: Some(position.as_model()),
                screen_offset: Vec2f::zero().as_model(),
                color: color.clone(),
            });
        }
    }

    pub fn add_world_line(&mut self, begin: Vec2f, end: Vec2f, color: Color) {
        self.line_vertices.push(ColoredVertex {
            world_pos: Some(begin.as_model()),
            screen_offset: Vec2f::zero().as_model(),
            color: color.clone(),
        });
        self.line_vertices.push(ColoredVertex {
            world_pos: Some(end.as_model()),
            screen_offset: Vec2f::zero().as_model(),
            color: color.clone(),
        });
    }

    pub fn send(&mut self, debug: &mut DebugInterface) {
        debug.send(model::DebugCommand::Clear {});
        if !self.triangle_vertices.is_empty() {
            debug.send(DebugCommand::Add {
                data: DebugData::Primitives {
                    vertices: self.triangle_vertices.clone(),
                    primitive_type: PrimitiveType::Triangles,
                }
            });
            self.triangle_vertices.clear();
        }
        if !self.line_vertices.is_empty() {
            debug.send(DebugCommand::Add {
                data: DebugData::Primitives {
                    vertices: self.line_vertices.clone(),
                    primitive_type: PrimitiveType::Lines,
                }
            });
            self.line_vertices.clear();
        }
        if !self.world_texts.is_empty() {
            for data in self.world_texts.iter() {
                debug.send(DebugCommand::Add { data: data.clone() });
            }
        }
        if !self.static_texts.is_empty() {
            for data in self.static_texts.iter() {
                debug.send(DebugCommand::Add { data: data.clone() });
            }
        }
        debug.send(model::DebugCommand::Flush {});
    }
}
