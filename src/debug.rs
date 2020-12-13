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
            size: 28.0,
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
            size: 28.0,
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

    pub fn add_static_rectangle(&mut self, min: Vec2f, max: Vec2f, color: Color) {
        let positions = &[
            min, Vec2f::new(min.x(), max.y()), max,
            min, Vec2f::new(max.x(), min.y()), max
        ];
        for position in positions.iter() {
            self.triangle_vertices.push(ColoredVertex {
                world_pos: None,
                screen_offset: position.as_model(),
                color: color.clone(),
            });
        }
    }

    pub fn add_time_series_i32<'v, I: Iterator<Item=(&'v Vec<i32>, Color)> + Clone>(&mut self, n: u32, name: String, values: I) {
        let max_len = values.clone()
            .map(|(v, _)| v.len())
            .max().unwrap_or(0);
        if max_len < 2 {
            return;
        }
        let min = values.clone()
            .map(|(v, _)| v.iter().min().cloned().unwrap_or(0))
            .min().unwrap_or(0);
        let max = values.clone()
            .map(|(v, _)| v.iter().max().cloned().unwrap_or(0))
            .max().unwrap_or(0);
        let width = self.state.window_size.x as f32 / 3.0;
        let height = self.state.window_size.y as f32 / 9.0;
        let shift = Vec2f::new(2.0 * self.state.window_size.x as f32 / 3.0 - 32.0, self.state.window_size.y as f32 - (height + 64.0) * (n + 1) as f32);
        let x_scale = width / (max_len - 1) as f32;
        let y_scale = height / (max - min).max(1) as f32;
        self.static_texts.push(DebugData::PlacedText {
            text: name,
            vertex: ColoredVertex {
                world_pos: None,
                screen_offset: Vec2f::new(shift.x() + width / 2.0, shift.y() + height + 16.0).as_model(),
                color: Color { a: 1.0, r: 1.0, g: 1.0, b: 1.0 },
            },
            alignment: 0.5,
            size: 28.0,
        });
        self.add_static_rectangle(shift, shift + Vec2f::new(width, height), Color { a: 0.1, r: 1.0, g: 1.0, b: 1.0 });
        for (v, color) in values {
            for i in 1..v.len() {
                self.line_vertices.push(ColoredVertex {
                    world_pos: None,
                    screen_offset: (shift + Vec2f::new((i - 1) as f32 * x_scale, (v[i - 1] - min) as f32 * y_scale)).as_model(),
                    color: color.clone(),
                });
                self.line_vertices.push(ColoredVertex {
                    world_pos: None,
                    screen_offset: (shift + Vec2f::new(i as f32 * x_scale, (v[i] - min) as f32 * y_scale)).as_model(),
                    color: color.clone(),
                });
            }
        }
    }

    pub fn add_world_cross(&mut self, center: Vec2f, size: f32, color: Color) {
        self.add_world_line(
            center - Vec2f::both(size),
            center + Vec2f::both(size),
            color.clone(),
        );
        self.add_world_line(
            center - Vec2f::both(size).left(),
            center + Vec2f::both(size).left(),
            color,
        );
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

pub fn get_player_color(alpha: f32, player_id: i32) -> Color {
    match player_id {
        1 => Color { a: alpha, r: 0.0, g: 0.0, b: 1.0 },
        2 => Color { a: alpha, r: 0.0, g: 1.0, b: 0.0 },
        3 => Color { a: alpha, r: 1.0, g: 0.0, b: 0.0 },
        4 => Color { a: alpha, r: 1.0, g: 1.0, b: 0.0 },
        _ => Color { a: alpha, r: 0.0, g: 0.0, b: 0.0 },
    }
}

pub fn color_from_heat(alpha: f32, mut value: f32) -> Color {
    value = value.max(0.0).min(1.0);
    if value < 0.25 {
        Color { a: alpha, r: 0.0, g: 4.0 * value, b: 1.0 }
    } else if value < 0.5 {
        Color { a: alpha, r: 0.0, g: 1.0, b: 1.0 - 4.0 * (value - 0.5) }
    } else if value < 0.75 {
        Color { a: alpha, r: 4.0 * (value - 0.5), g: 1.0, b: 0.0 }
    } else {
        Color { a: alpha, r: 1.0, g: 1.0 - 4.0 * (value - 0.75), b: 0.0 }
    }
}

#[derive(Default)]
pub struct Control {
    pub show_field: bool,
    pub show_group_field: bool,
    pub show_group_planner: bool,
    pub selected_group: usize,
    keyboard: Keyboard,
}

impl Control {
    pub fn update(&mut self, state: &DebugState) {
        self.keyboard.update(&state.pressed_keys);
        if self.keyboard.pressed_keys == &["LCtrl"] {
            if self.keyboard.pushed_keys == &["Q"] {
                self.show_field = !self.show_field;
                if self.show_field {
                    self.show_group_field = false;
                    self.show_group_planner = false;
                }
            } else if self.keyboard.pushed_keys == &["W"] {
                self.show_group_field = !self.show_group_field;
                if self.show_group_field {
                    self.show_field = false;
                    self.show_group_planner = false;
                }
            } else if self.keyboard.pushed_keys == &["R"] {
                self.show_group_planner = !self.show_group_planner;
                if self.show_group_planner {
                    self.show_field = false;
                    self.show_group_field = false;
                }
            }
        } else if self.keyboard.pressed_keys == &["LShift"] {
            if self.keyboard.pushed_keys == &["A"] {
                self.selected_group -= 1;
            } else if self.keyboard.pushed_keys == &["D"] {
                self.selected_group += 1;
            }
        }
    }
}

#[derive(Default)]
struct Keyboard {
    pressed_keys: Vec<String>,
    pushed_keys: Vec<String>,
}

impl Keyboard {
    fn update(&mut self, pressed_keys: &Vec<String>) {
        self.pushed_keys.clear();
        for old_key in self.pressed_keys.iter() {
            if !pressed_keys.contains(old_key) {
                self.pushed_keys.push(old_key.clone());
            }
        }
        self.pressed_keys.retain(|key| pressed_keys.contains(key));
        for new_key in pressed_keys.iter() {
            if !self.pressed_keys.contains(new_key) {
                self.pressed_keys.push(new_key.clone());
            }
        }
    }
}
