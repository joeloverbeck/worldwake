use std::collections::BTreeMap;

use egui::{Pos2, Vec2};
use rand_chacha::rand_core::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use worldwake_core::EntityId;

const LAYOUT_SIZE: f32 = 1_000.0;
const LAYOUT_CENTER: f32 = LAYOUT_SIZE / 2.0;
const AREA: f32 = LAYOUT_SIZE * LAYOUT_SIZE;
const ITERATIONS: usize = 200;
const INITIAL_TEMPERATURE: f32 = LAYOUT_SIZE / 10.0;
const MIN_DISTANCE: f32 = 0.01;

#[derive(Clone, Debug)]
pub struct PlaceLayout {
    pub positions: BTreeMap<EntityId, Pos2>,
    pub topology_fingerprint: u64,
}

impl PlaceLayout {
    pub fn compute(places: &[EntityId], edges: &[(EntityId, EntityId, u32)], seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut positions = BTreeMap::new();

        for place in sorted_places(places) {
            positions.insert(
                place,
                Pos2::new(
                    random_coordinate(&mut rng) * LAYOUT_SIZE,
                    random_coordinate(&mut rng) * LAYOUT_SIZE,
                ),
            );
        }

        if positions.len() > 1 {
            relax(&mut positions, &sorted_edges(edges));
            center_positions(&mut positions);
        }

        Self {
            positions,
            topology_fingerprint: topology_fingerprint(places, edges),
        }
    }
}

fn sorted_places(places: &[EntityId]) -> Vec<EntityId> {
    places
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn sorted_edges(edges: &[(EntityId, EntityId, u32)]) -> Vec<(EntityId, EntityId, u32)> {
    let mut sorted = edges.to_vec();
    sorted.sort_unstable();
    sorted
}

fn random_coordinate(rng: &mut ChaCha8Rng) -> f32 {
    rng.next_u32() as f32 / u32::MAX as f32
}

fn relax(positions: &mut BTreeMap<EntityId, Pos2>, edges: &[(EntityId, EntityId, u32)]) {
    let node_count = positions.len() as f32;
    let k_base = (AREA / node_count).sqrt();

    for iteration in 0..ITERATIONS {
        let mut displacements = positions
            .keys()
            .copied()
            .map(|id| (id, Vec2::ZERO))
            .collect::<BTreeMap<_, _>>();
        let ids = positions.keys().copied().collect::<Vec<_>>();

        for (left_index, left) in ids.iter().copied().enumerate() {
            for right in ids.iter().copied().skip(left_index + 1) {
                let delta = positions[&left] - positions[&right];
                let direction = normalized_or_default(delta);
                let force = k_base * k_base / delta.length().max(MIN_DISTANCE);
                *displacements.get_mut(&left).expect("displacement exists") += direction * force;
                *displacements.get_mut(&right).expect("displacement exists") -= direction * force;
            }
        }

        for &(from, to, travel_ticks) in edges {
            let (Some(from_pos), Some(to_pos)) = (positions.get(&from), positions.get(&to)) else {
                continue;
            };
            let delta = *from_pos - *to_pos;
            let direction = normalized_or_default(delta);
            let ideal_length = k_base * travel_ticks.max(1) as f32;
            let force = delta.length().powi(2) / ideal_length;
            *displacements.get_mut(&from).expect("displacement exists") -= direction * force;
            *displacements.get_mut(&to).expect("displacement exists") += direction * force;
        }

        let temperature = INITIAL_TEMPERATURE * (1.0 - iteration as f32 / ITERATIONS as f32);
        for (id, displacement) in displacements {
            let length = displacement.length();
            if length <= MIN_DISTANCE {
                continue;
            }
            let step = displacement / length * length.min(temperature);
            let position = positions.get_mut(&id).expect("position exists");
            position.x = (position.x + step.x).clamp(0.0, LAYOUT_SIZE);
            position.y = (position.y + step.y).clamp(0.0, LAYOUT_SIZE);
        }
    }
}

fn normalized_or_default(delta: Vec2) -> Vec2 {
    let length = delta.length();
    if length > MIN_DISTANCE {
        delta / length
    } else {
        Vec2::new(1.0, 0.0)
    }
}

fn center_positions(positions: &mut BTreeMap<EntityId, Pos2>) {
    let mut min = Pos2::new(f32::INFINITY, f32::INFINITY);
    let mut max = Pos2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);

    for position in positions.values() {
        min.x = min.x.min(position.x);
        min.y = min.y.min(position.y);
        max.x = max.x.max(position.x);
        max.y = max.y.max(position.y);
    }

    let center = Pos2::new((min.x + max.x) / 2.0, (min.y + max.y) / 2.0);
    let offset = Vec2::new(LAYOUT_CENTER - center.x, LAYOUT_CENTER - center.y);
    for position in positions.values_mut() {
        *position += offset;
    }
}

fn topology_fingerprint(places: &[EntityId], edges: &[(EntityId, EntityId, u32)]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325;
    mix_len(&mut hash, places.len() as u64);
    for place in sorted_places(places) {
        mix_entity(&mut hash, place);
    }
    mix_len(&mut hash, edges.len() as u64);
    for (from, to, ticks) in sorted_edges(edges) {
        mix_entity(&mut hash, from);
        mix_entity(&mut hash, to);
        mix_len(&mut hash, u64::from(ticks));
    }
    hash
}

fn mix_entity(hash: &mut u64, id: EntityId) {
    mix_len(hash, u64::from(id.slot));
    mix_len(hash, u64::from(id.generation));
}

fn mix_len(hash: &mut u64, value: u64) {
    *hash ^= value;
    *hash = hash.wrapping_mul(0x1000_0000_01b3);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fr_layout_is_deterministic() {
        let places = [entity(3), entity(1), entity(2), entity(4)];
        let edges = [
            (entity(1), entity(2), 2),
            (entity(2), entity(3), 3),
            (entity(3), entity(4), 1),
        ];

        let first = PlaceLayout::compute(&places, &edges, 42);
        let second = PlaceLayout::compute(&places, &edges, 42);

        assert_eq!(bit_positions(&first), bit_positions(&second));
        assert_eq!(first.topology_fingerprint, second.topology_fingerprint);
    }

    #[test]
    fn topology_fingerprint_stability() {
        let places = [entity(3), entity(1), entity(2)];
        let reversed_places = [entity(2), entity(3), entity(1)];
        let edges = [(entity(1), entity(2), 2), (entity(2), entity(3), 4)];
        let reordered_edges = [(entity(2), entity(3), 4), (entity(1), entity(2), 2)];

        let first = PlaceLayout::compute(&places, &edges, 7);
        let second = PlaceLayout::compute(&reversed_places, &reordered_edges, 99);

        assert_eq!(first.topology_fingerprint, second.topology_fingerprint);
    }

    #[test]
    fn topology_fingerprint_distinguishes_directed_edges() {
        let places = [entity(1), entity(2)];
        let outgoing = [(entity(1), entity(2), 2)];
        let incoming = [(entity(2), entity(1), 2)];

        let first = PlaceLayout::compute(&places, &outgoing, 7);
        let second = PlaceLayout::compute(&places, &incoming, 7);

        assert_ne!(first.topology_fingerprint, second.topology_fingerprint);
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn bit_positions(layout: &PlaceLayout) -> Vec<(EntityId, u32, u32)> {
        layout
            .positions
            .iter()
            .map(|(&id, pos)| (id, pos.x.to_bits(), pos.y.to_bits()))
            .collect()
    }
}
