use crate::{model::WorkspaceModel, policy::EffectiveConfig};
use depguard_types::Finding;

pub fn run_all(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    depguard_domain_checks::run_all(model, cfg, out)
}
