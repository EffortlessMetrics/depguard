use crate::model::WorkspaceModel;
use crate::policy::EffectiveConfig;
use depguard_check_catalog as check_catalog;
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
mod yanked_versions;

type CheckRunner = fn(&WorkspaceModel, &EffectiveConfig, &mut Vec<Finding>);

const RUNNERS: &[(&str, CheckRunner)] = &[
    (
        depguard_types::ids::CHECK_DEPS_NO_WILDCARDS,
        no_wildcards::run,
    ),
    (
        depguard_types::ids::CHECK_DEPS_PATH_REQUIRES_VERSION,
        path_requires_version::run,
    ),
    (
        depguard_types::ids::CHECK_DEPS_PATH_SAFETY,
        path_safety::run,
    ),
    (
        depguard_types::ids::CHECK_DEPS_WORKSPACE_INHERITANCE,
        workspace_inheritance::run,
    ),
    (
        depguard_types::ids::CHECK_DEPS_GIT_REQUIRES_VERSION,
        git_requires_version::run,
    ),
    (
        depguard_types::ids::CHECK_DEPS_DEV_ONLY_IN_NORMAL,
        dev_only_in_normal::run,
    ),
    (
        depguard_types::ids::CHECK_DEPS_DEFAULT_FEATURES_EXPLICIT,
        default_features_explicit::run,
    ),
    (
        depguard_types::ids::CHECK_DEPS_NO_MULTIPLE_VERSIONS,
        no_multiple_versions::run,
    ),
    (
        depguard_types::ids::CHECK_DEPS_OPTIONAL_UNUSED,
        optional_unused::run,
    ),
    (
        depguard_types::ids::CHECK_DEPS_YANKED_VERSIONS,
        yanked_versions::run,
    ),
];

pub fn run_all(model: &WorkspaceModel, cfg: &EffectiveConfig, out: &mut Vec<Finding>) {
    for (check_id, run) in RUNNERS {
        if check_catalog::is_check_available(check_id) {
            run(model, cfg, out);
        }
    }
}

#[cfg(test)]
mod tests;
