use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Chat,
    Embed,
    Classify,
    Summarize,
    Consolidate,
    VoiceStt,
    VoiceTts,
    ImageGenerate,
    VideoGenerate,
}
