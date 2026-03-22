use std::collections::BTreeSet;
use worldwake_core::{EntityId, InstitutionalBeliefRead, InstitutionalClaim, RecordData, RecordKind};

pub(crate) fn consulted_office_holder_read_for_record_data(
    record_data: &RecordData,
    office: EntityId,
) -> InstitutionalBeliefRead<Option<EntityId>> {
    if record_data.record_kind != RecordKind::OfficeRegister {
        return InstitutionalBeliefRead::Unknown;
    }

    let mut holders = BTreeSet::new();
    for entry in record_data
        .entries_newest_first()
        .take(record_data.max_entries_per_consult as usize)
    {
        if let InstitutionalClaim::OfficeHolder {
            office: entry_office,
            holder,
            ..
        } = entry.claim
        {
            if entry_office == office {
                holders.insert(holder);
            }
        }
    }

    match holders.len() {
        0 => InstitutionalBeliefRead::Unknown,
        1 => InstitutionalBeliefRead::Certain(
            *holders
                .iter()
                .next()
                .expect("single office-holder belief must exist"),
        ),
        _ => InstitutionalBeliefRead::Conflicted(holders.into_iter().collect()),
    }
}
