//! Navigation is derived from actual collision, with the same body clearance and step height as actors.
use crate::game::{
    map::MapConfig,
    player::player::{BODY_HEIGHT, BODY_RADIUS},
};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
};
#[derive(Resource, Default)]
pub struct Navigation {
    pub positions: Vec<Vec3>,
    pub edges: Vec<Vec<usize>>,
    pub destinations: Vec<usize>,
    pub patrol: Vec<usize>,
}
impl Navigation {
    pub fn nearest(&self, point: Vec3) -> Option<usize> {
        (0..self.positions.len()).min_by(|a, b| {
            self.positions[*a]
                .distance_squared(point)
                .total_cmp(&self.positions[*b].distance_squared(point))
        })
    }
    pub fn path(&self, start: usize, end: usize) -> Vec<usize> {
        self.path_avoiding(start, end, &[])
    }
    pub fn path_avoiding(&self, start: usize, end: usize, occupants: &[Vec3]) -> Vec<usize> {
        // Occupied floor costs more, but remains traversable so a narrow
        // corridor does not become permanently disconnected by one actor.
        let congestion: Vec<f32> = self
            .positions
            .iter()
            .map(|position| {
                if occupants.iter().any(|other| {
                    (position.y - other.y).abs() < 1.0
                        && position.xz().distance_squared(other.xz()) < 1.0
                }) {
                    8.0
                } else {
                    0.0
                }
            })
            .collect();
        let mut cost = vec![f32::INFINITY; self.positions.len()];
        let mut parent = vec![usize::MAX; cost.len()];
        let mut heap = BinaryHeap::new();
        cost[start] = 0.0;
        heap.push(Visit {
            cost: 0.0,
            node: start,
        });
        while let Some(Visit {
            cost: distance,
            node,
        }) = heap.pop()
        {
            if distance > cost[node] {
                continue;
            }
            if node == end {
                break;
            }
            for &next in &self.edges[node] {
                let candidate = distance
                    + self.positions[node].distance(self.positions[next])
                    + congestion[next];
                if candidate < cost[next] {
                    cost[next] = candidate;
                    parent[next] = node;
                    heap.push(Visit {
                        cost: candidate,
                        node: next,
                    });
                }
            }
        }
        if !cost[end].is_finite() {
            return Vec::new();
        }
        let mut path = vec![end];
        let mut current = end;
        while current != start {
            current = parent[current];
            path.push(current);
        }
        path.reverse();
        path
    }
}
#[derive(PartialEq)]
struct Visit {
    cost: f32,
    node: usize,
}
impl Eq for Visit {}
impl PartialOrd for Visit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Visit {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| self.node.cmp(&other.node))
    }
}

pub fn build(map: &MapConfig, physics: &RapierContext) -> Navigation {
    let Some((min, max)) = map.navigation_bounds else {
        return Navigation::default();
    };
    let transform = map.transform.to_transform();
    let min = transform.transform_point(min.to_vec3());
    let max = transform.transform_point(max.to_vec3());
    let spacing = 0.70;
    let width = ((max.x - min.x) / spacing).ceil() as i32;
    let depth = ((max.z - min.z) / spacing).ceil() as i32;
    let mut graph = Navigation::default();
    let mut cells: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    let clearance = Collider::cylinder(BODY_HEIGHT * 0.5 - 0.06, BODY_RADIUS + 0.015);
    let filter = QueryFilter::default().exclude_sensors();
    for x in 0..=width {
        for z in 0..=depth {
            let px = min.x + x as f32 * spacing;
            let pz = min.z + z as f32 * spacing;
            let mut top = max.y + 2.0;
            // Every vertical surface layer is examined, including the ground under bridges.
            for _ in 0..32 {
                if top < min.y - 1.0 {
                    break;
                }
                let Some((_, hit)) = physics.cast_ray_and_get_normal(
                    Vec3::new(px, top, pz),
                    -Vec3::Y,
                    top - min.y + 1.0,
                    true,
                    filter,
                ) else {
                    break;
                };
                let floor = hit.point;
                top = floor.y - 0.12;
                if hit.normal.y < 0.7 {
                    continue;
                }
                let center = floor + Vec3::Y * (BODY_HEIGHT * 0.5 + 0.03);
                let mut blocked = false;
                physics.intersections_with_shape(
                    center,
                    Quat::IDENTITY,
                    &clearance,
                    filter,
                    |_| {
                        blocked = true;
                        false
                    },
                );
                if blocked {
                    continue;
                }
                let index = graph.positions.len();
                graph.positions.push(floor);
                graph.edges.push(Vec::new());
                cells.entry((x, z)).or_default().push(index);
            }
        }
    }
    for (&(x, z), indices) in &cells {
        for &from in indices {
            for (dx, dz) in [(1, 0), (0, 1), (1, 1), (1, -1)] {
                if let Some(neighbors) = cells.get(&(x + dx, z + dz)) {
                    for &to in neighbors {
                        let start = graph.positions[from];
                        let end = graph.positions[to];
                        if (start.y - end.y).abs() > 0.48 {
                            continue;
                        }
                        // Lift by step allowance for horizontal clearance; ground continuity comes from adjacent supports.
                        let y = start.y.max(end.y) + BODY_HEIGHT * 0.5 + 0.05;
                        let origin = Vec3::new(start.x, y, start.z);
                        let delta = Vec3::new(end.x - start.x, 0.0, end.z - start.z);
                        if physics
                            .cast_shape(
                                origin,
                                Quat::IDENTITY,
                                delta,
                                &clearance,
                                ShapeCastOptions::with_max_time_of_impact(1.0),
                                filter,
                            )
                            .is_none()
                        {
                            graph.edges[from].push(to);
                            graph.edges[to].push(from);
                        }
                    }
                }
            }
        }
    }
    for point in &map.navigation {
        if let Some(node) = graph.nearest(transform.transform_point(point.position.to_vec3())) {
            graph.destinations.push(node);
        }
    }
    graph.patrol = map
        .patrol_destinations
        .iter()
        .copied()
        .filter(|&i| i < graph.destinations.len())
        .collect();
    info!(
        "Navigation: {} walkable nodes, {} directed connections",
        graph.positions.len(),
        graph.edges.iter().map(Vec::len).sum::<usize>()
    );
    if std::env::var_os("CSRS_CAPTURE").is_some() {
        let nodes: Vec<_> = graph.positions.iter().map(|p| [p.x, p.y, p.z]).collect();
        let data = format!(
            "{{\"nodes\":{nodes:?},\"edges\":{:?},\"destinations\":{:?}}}",
            graph.edges, graph.destinations
        );
        if let Err(error) = std::fs::write("/private/tmp/csrs-navigation.json", data.to_string()) {
            warn!("Navigation dump: {error}");
        }
        for spawn in &map.spawn_points {
            if let Some(start) = graph.nearest(transform.transform_point(spawn.position.to_vec3()))
            {
                info!(
                    "Spawn navigation {:?}: node {:?}, route lengths {:?}",
                    spawn.team,
                    graph.positions[start],
                    graph
                        .destinations
                        .iter()
                        .map(|&goal| graph.path(start, goal).len())
                        .collect::<Vec<_>>()
                );
            }
        }
    }
    graph
}
