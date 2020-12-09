use crate::my_strategy::{MovingAverageSpeed, World};

pub struct Stats {
    player_id: i32,
    resource_rate: MovingAverageSpeed<i32>,
    score_rate: MovingAverageSpeed<i32>,
}

impl Stats {
    pub fn new(player_id: i32) -> Self {
        Self {
            player_id,
            resource_rate: MovingAverageSpeed::new(50, 50),
            score_rate: MovingAverageSpeed::new(50, 50),
        }
    }

    pub fn update(&mut self, world: &World) {
        let player = world.get_player(self.player_id);
        self.resource_rate.add(player.resource, world.current_tick());
        self.score_rate.add(player.score, world.current_tick());
    }

    pub fn resource_rate(&self) -> f32 {
        self.resource_rate.get()
    }

    pub fn score_rate(&self) -> f32 {
        self.score_rate.get()
    }
}
