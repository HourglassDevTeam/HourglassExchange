use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WsTrade {
    instId: String,
    side: String,
    px: String,
    ts: String,
}