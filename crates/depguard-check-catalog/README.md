# depguard-check-catalog

Central check metadata and feature-gating for depguard checks.

- One source of truth for check IDs and defaults per profile.
- Compile-time feature availability (`check-*`) for each check.
- Shared profile preset table used by settings.
- BDD feature coverage mapping (`bdd_feature_file`) to keep check coverage explicit.
