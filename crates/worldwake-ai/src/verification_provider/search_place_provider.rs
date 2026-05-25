use super::{VerificationCandidate, VerificationContext, VerificationNeed, VerificationRejection};

pub fn try_build(
    _need: &VerificationNeed,
    _ctx: &VerificationContext<'_>,
) -> Result<VerificationCandidate, VerificationRejection> {
    Err(VerificationRejection::BreachClassMismatch)
}
