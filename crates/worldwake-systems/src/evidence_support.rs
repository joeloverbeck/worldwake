use worldwake_core::{
    EntityId, EvidenceEntry, EvidenceEntryId, EvidenceKind, WorldError, WorldTxn,
};

pub(crate) fn emit_evidence(
    txn: &mut WorldTxn<'_>,
    place: EntityId,
    kind: EvidenceKind,
    decay_ticks: u32,
) -> Result<(), WorldError> {
    let mut scene = txn
        .get_component_scene_evidence(place)
        .cloned()
        .unwrap_or_default();
    let entry_id = EvidenceEntryId(scene.next_entry_id);
    scene.next_entry_id += 1;
    scene.evidence.push(EvidenceEntry {
        id: entry_id,
        kind,
        created_at: txn.tick(),
        decay_ticks,
    });
    txn.set_component_scene_evidence(place, scene)
}

#[cfg(test)]
mod tests {
    use super::emit_evidence;
    use worldwake_core::{
        build_prototype_world, prototype_place_entity, CauseRef, DisturbanceKind, EntityId,
        EventLog, EvidenceEntryId, EvidenceKind, PrototypePlace, Tick, VisibilitySpec, WitnessData,
        World, WorldTxn,
    };

    fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            Tick(tick),
            CauseRef::Bootstrap,
            None,
            None,
            VisibilitySpec::SamePlace,
            WitnessData::default(),
        )
    }

    #[test]
    fn emit_evidence_allocates_unique_ids_and_accumulates_entries() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = prototype_place_entity(PrototypePlace::VillageSquare);

        {
            let mut txn = new_txn(&mut world, 5);
            emit_evidence(
                &mut txn,
                place,
                EvidenceKind::DisturbanceMarker {
                    place,
                    kind: DisturbanceKind::ForcedEntry,
                    created_at: Tick(5),
                },
                50,
            )
            .unwrap();
            emit_evidence(
                &mut txn,
                place,
                EvidenceKind::MovementTrace {
                    entity: EntityId {
                        slot: 99,
                        generation: 0,
                    },
                    departed_from: place,
                    direction: prototype_place_entity(PrototypePlace::ForestPath),
                    observed_at: Tick(5),
                },
                30,
            )
            .unwrap();
            let mut log = EventLog::new();
            let _ = txn.commit(&mut log);
        }

        let scene = world
            .get_component_scene_evidence(place)
            .expect("scene evidence should be present");
        assert_eq!(scene.evidence.len(), 2);
        assert_eq!(scene.evidence[0].id, EvidenceEntryId(0));
        assert_eq!(scene.evidence[1].id, EvidenceEntryId(1));
        assert_eq!(scene.next_entry_id, 2);
    }
}
