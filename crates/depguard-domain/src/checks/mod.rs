use crate::model::WorkspaceModel;
use crate::policy::EffectiveConfig;
use depguard_types::Finding;

mod no_wildcards;
mod path_requires_version;
mod path_safety;
mod utils;
mod workspace_inheritance;

pub fn run_all(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    no_wildcards::run(model, cfg, out);
    path_requires_version::run(model, cfg, out);
    path_safety::run(model, cfg, out);
    workspace_inheritance::run(model, cfg, out);
}
