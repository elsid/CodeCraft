use crate::my_strategy::{position_to_index, Rect, Vec2i};

#[derive(Debug)]
pub struct ReachabilityMap {
    map_size: usize,
    reachable: Vec<bool>,
}

impl ReachabilityMap {
    pub fn new(map_size: usize) -> Self {
        Self {
            map_size,
            reachable: std::iter::repeat(false).take(map_size * map_size).collect(),
        }
    }

    pub fn is_reachable(&self, dst: Vec2i) -> bool {
        self.reachable[position_to_index(dst, self.map_size)]
    }

    pub fn update(&mut self, start: Vec2i, is_passable: &Vec<bool>) {
        for value in self.reachable.iter_mut() {
            *value = false;
        }
        let start_index = position_to_index(start, self.map_size);
        self.reachable[start_index] = true;

        let mut discovered: Vec<Vec2i> = Vec::new();
        discovered.push(start);

        const EDGES: &[Vec2i] = &[
            Vec2i::only_x(1),
            Vec2i::only_x(-1),
            Vec2i::only_y(1),
            Vec2i::only_y(-1),
        ];

        let bounds = Rect::new(Vec2i::zero(), Vec2i::both(self.map_size as i32));

        while let Some(node_position) = discovered.pop() {
            let node_index = position_to_index(node_position, self.map_size);
            self.reachable[node_index] = true;
            for &shift in EDGES.iter() {
                let neighbor_position = node_position + shift;
                if !bounds.contains(neighbor_position) {
                    continue;
                }
                let neighbor_index = position_to_index(neighbor_position, self.map_size);
                if self.reachable[neighbor_index] || !is_passable[neighbor_index] {
                    continue;
                }
                self.reachable[neighbor_index] = true;
                discovered.push(neighbor_position);
            }
        }
    }
}

pub fn visit_reversed_shortest_path<F: FnMut(usize)>(src: usize, dst: usize, backtrack: &Vec<usize>, mut visit: F) -> bool {
    let mut index = dst;
    loop {
        let prev = backtrack[index];
        if prev == index {
            return false;
        }
        visit(index);
        if prev == src {
            break;
        }
        index = prev;
    }
    true
}
