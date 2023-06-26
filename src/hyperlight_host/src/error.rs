use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub(crate) struct HyperlightError {
    pub(crate) message: String,
    pub(crate) source: String,
}
