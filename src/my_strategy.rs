use std::collections::HashMap;

#[allow(unused_imports)]
pub use bot::*;
#[allow(unused_imports)]
pub use entity::*;
#[allow(unused_imports)]
pub use entity_type::*;
#[allow(unused_imports)]
pub use groups::*;
#[allow(unused_imports)]
pub use map::*;
#[allow(unused_imports)]
pub use math::*;
#[allow(unused_imports)]
pub use moving_average::*;
#[allow(unused_imports)]
pub use positionable::*;
#[allow(unused_imports)]
pub use roles::*;
#[allow(unused_imports)]
pub use stats::*;
#[allow(unused_imports)]
pub use tasks::*;
#[allow(unused_imports)]
pub use vec2::*;
#[allow(unused_imports)]
pub use world::*;

use super::DebugInterface;

#[path = "debug.rs"]
pub mod debug;

#[path = "groups.rs"]
pub mod groups;

#[path = "tasks.rs"]
pub mod tasks;

#[path = "entity_type.rs"]
pub mod entity_type;

#[path = "moving_average.rs"]
pub mod moving_average;

#[path = "stats.rs"]
pub mod stats;

#[path = "map.rs"]
pub mod map;

#[path = "roles.rs"]
pub mod roles;

#[path = "positionable.rs"]
pub mod positionable;

#[path = "entity.rs"]
pub mod entity;

#[path = "math.rs"]
pub mod math;

#[path = "vec2.rs"]
pub mod vec2;

#[path = "world.rs"]
pub mod world;

#[path = "bot.rs"]
pub mod bot;

pub struct MyStrategy {
    bot: Option<Bot>,
}

impl MyStrategy {
    pub fn new() -> Self {
        Self {
            bot: None,
        }
    }

    pub fn get_action(
        &mut self,
        player_view: &model::PlayerView,
        _debug_interface: Option<&mut DebugInterface>,
    ) -> model::Action {
        if self.bot.is_none() {
            self.bot = Some(Bot::new(World::new(&player_view)));
        }
        self.bot.as_mut()
            .map(|v| v.get_action(player_view))
            .unwrap_or_else(|| model::Action {
                entity_actions: HashMap::new(),
            })
    }

    pub fn debug_update(
        &mut self,
        _player_view: &model::PlayerView,
        debug_interface: &mut DebugInterface,
    ) {
        debug_interface.send(model::DebugCommand::Clear {});
        let state = debug_interface.get_state();
        self.bot.as_mut().map(|v| v.debug_update(&state, debug_interface));
    }
}
