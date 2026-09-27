use crate::reason::{Failure, Skip};
use crate::record::Record;
use crate::service::Reply;

pub(crate) struct Decision {
    pub(crate) record: Record,
    pub(crate) outcome: Outcome,
}

pub(crate) enum Outcome {
    Answered { reply: Reply, truncated: bool },
    Skipped(Skip),
    Failed(Failure),
}
