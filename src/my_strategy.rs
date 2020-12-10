use std::collections::HashMap;

#[allow(unused_imports)]
pub use bot::*;
#[allow(unused_imports)]
pub use config::*;
#[allow(unused_imports)]
pub use entity::*;
#[allow(unused_imports)]
pub use entity_type::*;
#[allow(unused_imports)]
pub use field::*;
#[allow(unused_imports)]
pub use groups::*;
#[allow(unused_imports)]
pub use map::*;
#[allow(unused_imports)]
pub use moving_average::*;
#[allow(unused_imports)]
pub use positionable::*;
#[allow(unused_imports)]
pub use rect::*;
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

#[path = "field.rs"]
pub mod field;

#[path = "config.rs"]
pub mod config;

#[path = "rect.rs"]
pub mod rect;

#[cfg(feature = "enable_debug")]
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

#[path = "vec2.rs"]
pub mod vec2;

#[path = "world.rs"]
pub mod world;

#[path = "bot.rs"]
pub mod bot;

pub struct MyStrategy {
    bot: Option<Bot>,
    #[cfg(feature = "write_player_view")]
    player_view_file: std::fs::File,
    #[cfg(feature = "max_tick")]
    max_tick: i32,
}

impl MyStrategy {
    pub fn new() -> Self {
        Self {
            bot: None,
            #[cfg(feature = "write_player_view")]
            player_view_file: std::fs::File::create("player_view.json").unwrap(),
            #[cfg(feature = "max_tick")]
            max_tick: std::env::var("MAX_TICK").map(|v| v.parse::<i32>().unwrap_or(std::i32::MAX)).unwrap_or(std::i32::MAX),
        }
    }

    pub fn get_action(
        &mut self,
        player_view: &model::PlayerView,
        _debug_interface: Option<&mut DebugInterface>,
    ) -> model::Action {
        #[cfg(feature = "max_tick")]
        if player_view.current_tick > self.max_tick {
            std::process::exit(0);
        }
        #[cfg(feature = "write_player_view")]
            self.write_player_view(player_view);
        if self.bot.is_none() {
            let config = get_config();
            #[cfg(feature = "print_config")]
            println!("{}", serde_json::to_string(&config).unwrap());
            self.bot = Some(Bot::new(World::new(&player_view), config));
        }
        self.bot.as_mut()
            .map(|v| v.get_action(player_view))
            .unwrap_or_else(|| model::Action {
                entity_actions: HashMap::new(),
            })
    }

    #[cfg(not(feature = "enable_debug"))]
    pub fn debug_update(
        &mut self,
        _player_view: &model::PlayerView,
        _debug_interface: &mut DebugInterface,
    ) {}

    #[cfg(feature = "enable_debug")]
    pub fn debug_update(
        &mut self,
        _player_view: &model::PlayerView,
        debug_interface: &mut DebugInterface,
    ) {
        let state = debug_interface.get_state();
        self.bot.as_mut().map(|v| v.debug_update(&state, debug_interface));
    }

    #[cfg(feature = "write_player_view")]
    fn write_player_view(&mut self, player_view: &model::PlayerView) {
        use std::io::Write;
        serde_json::to_writer(&mut self.player_view_file, &player_view).unwrap();
        self.player_view_file.write(b"\n").unwrap();
    }
}

#[cfg(not(feature = "read_config"))]
fn get_config() -> Config {
    Config::new()
}

#[cfg(feature = "read_config")]
fn get_config() -> Config {
    serde_json::from_str(
        std::fs::read_to_string(
            std::env::var("CONFIG").expect("CONFIG env is not found")
        ).expect("Can't read config file").as_str()
    ).expect("Can't parse config file")
}
