//! Analysis modules — pure heuristics that turn indexed data into signals
//! consumed by the daemon's endpoints and scheduled jobs. Kept separate
//! from `indexer` (which owns *what* to index) and `tasks` (which owns
//! *when* to run things) so the heuristics stay unit-testable.

pub mod doc_drift;
pub mod insight_copy;
pub mod metric_narrative;
pub mod project_icon;
pub mod rule_consolidation;
