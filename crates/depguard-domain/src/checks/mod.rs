use crate::model::WorkspaceModel;
use crate::policy::EffectiveConfig;
use depguard_types::Finding;

mod default_features_explicit;
mod dev_only_in_normal;
mod git_requires_version;
mod no_multiple_versions;
mod no_wildcards;
mod optional_unused;
mod path_requires_version;
mod path_safety;
mod utils;
mod workspace_inheritance;

pub fn run_all(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    no_wildcards::run(model, cfg, out);
    path_requires_version::run(model, cfg, out);
    path_safety::run(model, cfg, out);
    workspace_inheritance::run(model, cfg, out);
    git_requires_version::run(model, cfg, out);
    dev_only_in_normal::run(model, cfg, out);
    default_features_explicit::run(model, cfg, out);
    no_multiple_versions::run(model, cfg, out);
    optional_unused::run(model, cfg, out);
}
