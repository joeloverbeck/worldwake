use std::collections::BTreeMap;

use egui::{
    Align2, Color32, FontId, Pos2, Rect, Scene, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
};
use worldwake_core::{ControlSource, EntityId, Permille, PlaceTag};

use crate::snapshot::{AgentPosition, AgentView, FrameSnapshot};
use crate::tooltip;

const PLACE_SIZE: Vec2 = Vec2::new(150.0, 78.0);
const PLACE_RADIUS: u8 = 8;
const AGENT_RADIUS: f32 = 10.0;
const AGENT_RING_RADIUS: f32 = 22.0;
const AGENT_HIT_RADIUS: f32 = 14.0;
const EDGE_OFFSET: f32 = 6.0;
const SCENE_SIZE: Vec2 = Vec2::new(1_100.0, 1_100.0);

pub fn draw_canvas(
    ui: &mut Ui,
    snapshot: &FrameSnapshot,
    scene_rect: &mut Rect,
    selected_agent: &mut Option<EntityId>,
    hovered_agent: &mut Option<EntityId>,
) {
    let draw_data = CanvasDrawData::from_snapshot(snapshot);
    *hovered_agent = None;

    Scene::new()
        .zoom_range(0.15..=4.0)
        .max_inner_size(SCENE_SIZE)
        .show(ui, scene_rect, |ui| {
            ui.set_min_size(SCENE_SIZE);
            draw_edges(ui, snapshot, &draw_data);
            draw_places(ui, snapshot);
            draw_agents(ui, snapshot, &draw_data, selected_agent, hovered_agent);
        });
}

#[derive(Clone, Debug)]
struct CanvasDrawData {
    agent_positions: BTreeMap<EntityId, Pos2>,
    edge_offsets: Vec<Vec2>,
}

impl CanvasDrawData {
    fn from_snapshot(snapshot: &FrameSnapshot) -> Self {
        Self {
            agent_positions: agent_draw_positions(snapshot),
            edge_offsets: edge_offsets(snapshot),
        }
    }
}

fn draw_edges(ui: &mut Ui, snapshot: &FrameSnapshot, draw_data: &CanvasDrawData) {
    for (edge, offset) in snapshot.edges.iter().zip(&draw_data.edge_offsets) {
        let (Some(from), Some(to)) = (
            snapshot.places.get(&edge.from),
            snapshot.places.get(&edge.to),
        ) else {
            continue;
        };
        let from_pos = from.position + *offset;
        let to_pos = to.position + *offset;

        if edge.from == edge.to {
            let center = from.position + Vec2::new(0.0, -PLACE_SIZE.y * 0.7);
            let points = arc_points(center, 36.0, 0.15, std::f32::consts::TAU * 0.9);
            ui.painter().extend(Shape::dashed_line(
                &points,
                Stroke::new(3.0, Color32::GRAY),
                8.0,
                4.0,
            ));
            draw_edge_label(ui, center + Vec2::new(0.0, -34.0), edge.travel_ticks);
        } else {
            ui.painter().extend(Shape::dashed_line(
                &[from_pos, to_pos],
                Stroke::new(3.0, Color32::GRAY),
                8.0,
                4.0,
            ));
            draw_edge_label(ui, from_pos.lerp(to_pos, 0.5), edge.travel_ticks);
        }
    }
}

fn draw_edge_label(ui: &mut Ui, center: Pos2, travel_ticks: u32) {
    let text = format!("{travel_ticks} ticks");
    let font = FontId::proportional(12.0);
    let galley = ui
        .painter()
        .layout_no_wrap(text.clone(), font.clone(), Color32::WHITE);
    let rect = Rect::from_center_size(center, galley.size() + Vec2::new(12.0, 6.0));
    ui.painter()
        .rect_filled(rect, 5, Color32::from_rgba_unmultiplied(18, 18, 22, 230));
    ui.painter().text(
        center,
        Align2::CENTER_CENTER,
        text,
        font,
        Color32::from_rgb(230, 230, 235),
    );
}

fn draw_places(ui: &mut Ui, snapshot: &FrameSnapshot) {
    for place in snapshot.places.values() {
        let rect = Rect::from_center_size(place.position, PLACE_SIZE);
        let stroke_color = place
            .tags
            .first()
            .copied()
            .map_or(Color32::GRAY, place_tag_color);
        ui.painter()
            .rect_filled(rect, PLACE_RADIUS, Color32::from_rgb(36, 36, 42));
        ui.painter().rect_stroke(
            rect,
            PLACE_RADIUS,
            Stroke::new(1.5, stroke_color),
            StrokeKind::Outside,
        );
        ui.painter().text(
            rect.left_top() + Vec2::new(10.0, 9.0),
            Align2::LEFT_TOP,
            &place.name,
            FontId::proportional(14.0),
            Color32::from_rgb(238, 238, 242),
        );

        let mut tag_left = rect.left() + 10.0;
        let tag_top = rect.top() + 36.0;
        for tag in &place.tags {
            let label = place_tag_label(*tag);
            let font = FontId::proportional(10.0);
            let galley =
                ui.painter()
                    .layout_no_wrap(label.to_string(), font.clone(), Color32::WHITE);
            let pill = Rect::from_min_size(
                Pos2::new(tag_left, tag_top),
                galley.size() + Vec2::new(10.0, 5.0),
            );
            ui.painter()
                .rect_filled(pill, 4, place_tag_color(*tag).gamma_multiply(0.45));
            ui.painter().text(
                pill.center(),
                Align2::CENTER_CENTER,
                label,
                font,
                Color32::from_rgb(244, 244, 247),
            );
            tag_left = pill.right() + 5.0;
        }
    }
}

fn draw_agents(
    ui: &mut Ui,
    snapshot: &FrameSnapshot,
    draw_data: &CanvasDrawData,
    selected_agent: &mut Option<EntityId>,
    hovered_agent: &mut Option<EntityId>,
) {
    for (&agent_id, agent) in &snapshot.agents {
        let Some(position) = draw_data.agent_positions.get(&agent_id).copied() else {
            continue;
        };
        let fill = agent_fill(agent);
        let stroke = if *selected_agent == Some(agent_id) {
            Stroke::new(2.5, Color32::WHITE)
        } else {
            Stroke::new(1.0, Color32::from_rgb(18, 18, 22))
        };
        ui.painter().circle_filled(position, AGENT_RADIUS, fill);
        ui.painter().circle_stroke(position, AGENT_RADIUS, stroke);

        if !agent.alive {
            draw_dead_agent_stripes(ui, position);
        }
        if let AgentPosition::InTransit { from, to, .. } = agent.position {
            draw_agent_chevron(ui, position, snapshot, from, to);
        }

        ui.painter().text(
            position + Vec2::new(0.0, 14.0),
            Align2::CENTER_TOP,
            elide_agent_label(&agent.name),
            FontId::proportional(11.0),
            Color32::from_rgb(235, 235, 238),
        );

        let hit_rect = Rect::from_center_size(position, Vec2::splat(AGENT_HIT_RADIUS * 2.0));
        let response = ui
            .interact(hit_rect, ui.id().with(agent_id), Sense::click())
            .on_hover_ui(|ui| tooltip::show_tooltip(ui, snapshot, agent));
        if response.hovered() {
            *hovered_agent = Some(agent_id);
        }
        if response.clicked() {
            *selected_agent = Some(agent_id);
        }
    }
}

fn draw_dead_agent_stripes(ui: &mut Ui, center: Pos2) {
    let stroke = Stroke::new(1.5, Color32::from_rgb(90, 90, 96));
    for offset in [-8.0, -3.0, 2.0, 7.0] {
        ui.painter().line_segment(
            [
                center + Vec2::new(offset - 5.0, -8.0),
                center + Vec2::new(offset + 5.0, 8.0),
            ],
            stroke,
        );
    }
}

fn draw_agent_chevron(
    ui: &mut Ui,
    center: Pos2,
    snapshot: &FrameSnapshot,
    from: EntityId,
    to: EntityId,
) {
    let Some(from_pos) = snapshot.places.get(&from).map(|place| place.position) else {
        return;
    };
    let Some(to_pos) = snapshot.places.get(&to).map(|place| place.position) else {
        return;
    };
    let direction = normalized_or_default(to_pos - from_pos);
    let tip = center + direction * (AGENT_RADIUS + 8.0);
    let normal = Vec2::new(-direction.y, direction.x);
    let tail = center + direction * (AGENT_RADIUS + 1.0);
    ui.painter().add(Shape::convex_polygon(
        vec![tip, tail + normal * 4.0, tail - normal * 4.0],
        Color32::from_rgb(245, 245, 248),
        Stroke::NONE,
    ));
}

fn agent_draw_positions(snapshot: &FrameSnapshot) -> BTreeMap<EntityId, Pos2> {
    let mut by_place: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
    let mut positions = BTreeMap::new();

    for (&agent_id, agent) in &snapshot.agents {
        match agent.position {
            AgentPosition::AtPlace(place_id) => {
                by_place.entry(place_id).or_default().push(agent_id);
            }
            AgentPosition::InTransit {
                from, to, progress, ..
            } => {
                if let Some(position) = transit_position(snapshot, from, to, progress) {
                    positions.insert(agent_id, position);
                }
            }
        }
    }

    for (place_id, mut agent_ids) in by_place {
        agent_ids.sort_unstable();
        let Some(place) = snapshot.places.get(&place_id) else {
            continue;
        };
        for (agent_id, angle) in fan_out_angles(&agent_ids) {
            let offset = Vec2::angled(angle) * AGENT_RING_RADIUS;
            positions.insert(agent_id, place.position + offset);
        }
    }

    positions
}

fn transit_position(
    snapshot: &FrameSnapshot,
    from: EntityId,
    to: EntityId,
    progress: Permille,
) -> Option<Pos2> {
    let from_pos = snapshot.places.get(&from)?.position;
    let to_pos = snapshot.places.get(&to)?.position;
    Some(from_pos.lerp(to_pos, f32::from(progress.value()) / 1000.0))
}

fn fan_out_angles(agent_ids: &[EntityId]) -> BTreeMap<EntityId, f32> {
    let count = agent_ids.len();
    agent_ids
        .iter()
        .copied()
        .enumerate()
        .map(|(index, id)| {
            (
                id,
                index as f32 * std::f32::consts::TAU / count.max(1) as f32,
            )
        })
        .collect()
}

fn edge_offsets(snapshot: &FrameSnapshot) -> Vec<Vec2> {
    let mut grouped: BTreeMap<(EntityId, EntityId), usize> = BTreeMap::new();
    for edge in &snapshot.edges {
        *grouped.entry((edge.from, edge.to)).or_default() += 1;
    }

    let mut seen: BTreeMap<(EntityId, EntityId), usize> = BTreeMap::new();
    let mut offsets = Vec::with_capacity(snapshot.edges.len());
    for edge in &snapshot.edges {
        let key = (edge.from, edge.to);
        let index = *seen.entry(key).and_modify(|value| *value += 1).or_default();
        let count = grouped[&key];
        let offset = if edge.from == edge.to || count <= 1 {
            Vec2::ZERO
        } else {
            let from = snapshot.places.get(&edge.from).map(|place| place.position);
            let to = snapshot.places.get(&edge.to).map(|place| place.position);
            match (from, to) {
                (Some(from), Some(to)) => {
                    let normal = perpendicular_normal(from, to);
                    let centered_index = index as f32 - (count.saturating_sub(1)) as f32 / 2.0;
                    normal * centered_index * EDGE_OFFSET
                }
                _ => Vec2::ZERO,
            }
        };
        offsets.push(offset);
    }
    offsets
}

fn perpendicular_normal(from: Pos2, to: Pos2) -> Vec2 {
    let direction = normalized_or_default(to - from);
    Vec2::new(-direction.y, direction.x)
}

fn normalized_or_default(delta: Vec2) -> Vec2 {
    let length = delta.length();
    if length > f32::EPSILON {
        delta / length
    } else {
        Vec2::new(1.0, 0.0)
    }
}

fn arc_points(center: Pos2, radius: f32, start: f32, end: f32) -> Vec<Pos2> {
    (0..=24)
        .map(|step| {
            let t = step as f32 / 24.0;
            let angle = start + (end - start) * t;
            center + Vec2::angled(angle) * radius
        })
        .collect()
}

fn agent_fill(agent: &AgentView) -> Color32 {
    if !agent.alive {
        return Color32::from_rgb(130, 130, 138);
    }
    match agent.control {
        ControlSource::Ai => Color32::from_rgb(96, 165, 250),
        ControlSource::Human => Color32::from_rgb(245, 190, 80),
        ControlSource::None => Color32::from_rgb(150, 154, 164),
    }
}

fn elide_agent_label(name: &str) -> String {
    const MAX_CHARS: usize = 16;
    let mut chars = name.chars();
    let prefix = chars.by_ref().take(MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{prefix}...")
    } else {
        prefix
    }
}

fn place_tag_label(tag: PlaceTag) -> &'static str {
    match tag {
        PlaceTag::Village => "Village",
        PlaceTag::Farm => "Farm",
        PlaceTag::Store => "Store",
        PlaceTag::Inn => "Inn",
        PlaceTag::Hall => "Hall",
        PlaceTag::Barracks => "Barracks",
        PlaceTag::Latrine => "Latrine",
        PlaceTag::Crossroads => "Crossroads",
        PlaceTag::Forest => "Forest",
        PlaceTag::Camp => "Camp",
        PlaceTag::Road => "Road",
        PlaceTag::Trail => "Trail",
        PlaceTag::Field => "Field",
        PlaceTag::Gate => "Gate",
    }
}

fn place_tag_color(tag: PlaceTag) -> Color32 {
    match tag {
        PlaceTag::Village => Color32::from_rgb(220, 174, 91),
        PlaceTag::Farm => Color32::from_rgb(118, 184, 91),
        PlaceTag::Store => Color32::from_rgb(123, 178, 232),
        PlaceTag::Inn => Color32::from_rgb(209, 138, 95),
        PlaceTag::Hall => Color32::from_rgb(186, 151, 224),
        PlaceTag::Barracks => Color32::from_rgb(212, 96, 96),
        PlaceTag::Latrine => Color32::from_rgb(139, 132, 116),
        PlaceTag::Crossroads => Color32::from_rgb(186, 186, 190),
        PlaceTag::Forest => Color32::from_rgb(76, 154, 112),
        PlaceTag::Camp => Color32::from_rgb(224, 128, 72),
        PlaceTag::Road => Color32::from_rgb(168, 158, 145),
        PlaceTag::Trail => Color32::from_rgb(132, 174, 112),
        PlaceTag::Field => Color32::from_rgb(190, 194, 92),
        PlaceTag::Gate => Color32::from_rgb(108, 142, 204),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::PlaceLayout;
    use crate::snapshot::build_snapshot;
    use worldwake_ai::AgentTickDriver;
    use worldwake_cli::scenario::{load_scenario_file, spawn_scenario_ignoring_lints};
    use worldwake_core::Tick;

    #[test]
    fn canvas_smoke_no_panic_on_baseline_scenario() {
        let snapshot = baseline_snapshot();
        let ctx = egui::Context::default();
        let mut scene_rect = Rect::from_min_size(Pos2::ZERO, SCENE_SIZE);
        let mut selected_agent = None;
        let mut hovered_agent = None;

        let _ = ctx.run_ui(Default::default(), |ui| {
            draw_canvas(
                ui,
                &snapshot,
                &mut scene_rect,
                &mut selected_agent,
                &mut hovered_agent,
            );
        });

        let agent_positions = agent_draw_positions(&snapshot);
        assert_eq!(agent_positions.len(), snapshot.agents.len());
    }

    #[test]
    fn agent_fan_out_angles_are_btreemap_stable() {
        let agents = [entity(30), entity(10), entity(20)];
        let sorted = agents
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        let angles = fan_out_angles(&sorted);
        let ordered = angles.into_iter().collect::<Vec<_>>();

        assert_eq!(ordered[0].0, entity(10));
        assert_eq!(ordered[1].0, entity(20));
        assert_eq!(ordered[2].0, entity(30));
        assert_eq!(ordered[0].1, 0.0);
        assert!((ordered[1].1 - std::f32::consts::TAU / 3.0).abs() < f32::EPSILON);
        assert!((ordered[2].1 - std::f32::consts::TAU * 2.0 / 3.0).abs() < f32::EPSILON);
        assert!(
            (ordered.iter().map(|(_, angle)| angle).sum::<f32>() - std::f32::consts::TAU).abs()
                < 0.000_001
        );
    }

    fn baseline_snapshot() -> FrameSnapshot {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("scenarios/survival-baseline.ron");
        let def = load_scenario_file(&scenario).expect("baseline scenario loads");
        let spawned = spawn_scenario_ignoring_lints(&def).expect("baseline scenario spawns");
        let world = spawned.state.world();
        let places = world.topology().place_ids().collect::<Vec<_>>();
        let mut edges = Vec::new();
        for place in world.topology().place_ids() {
            for edge_id in world.topology().outgoing_edges(place) {
                let edge = world.topology().edge(*edge_id).expect("edge resolves");
                edges.push((edge.from(), edge.to(), edge.travel_time_ticks()));
            }
        }
        let layout = PlaceLayout::compute(&places, &edges, 0);
        let driver = AgentTickDriver::new();
        build_snapshot(world, spawned.state.scheduler(), &driver, &layout, Tick(0))
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }
}
