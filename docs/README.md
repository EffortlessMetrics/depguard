# Documentation index

## Problem
The repository has multiple documentation entry points (architecture, checks, config, CI, testing) and users need a clear starting point.

## How docs are organized
- **Tutorials**: step-by-step onboarding
  - [docs/quickstart.md](quickstart.md)
  - [docs/ci-integration.md](ci-integration.md)
- **How-to guides**: practical tasks and workflows
  - [docs/config.md](config.md)
  - [docs/troubleshooting.md](troubleshooting.md)
  - [docs/testing.md](testing.md)
- **Reference**: design and contracts
  - [docs/architecture.md](architecture.md)
  - [docs/output-contract.md](output-contract.md)
  - [docs/microcrates.md](microcrates.md)
  - [docs/design.md](design.md)
  - [docs/implementation.md](implementation.md)
  - [docs/implementation-plan.md](implementation-plan.md)
- **Roadmap**
  - [docs/roadmap.md](roadmap.md)
  - [docs/org-rollout.md](org-rollout.md)
- **Explanation**: rationale and boundaries
  - [docs/checks.md](checks.md)
  - [docs/requirements.md](requirements.md)

## Contributing guidance
- Prefer adding docs next to behavior changes.
- Keep examples executable and deterministic.
- Link new guidance to the relevant crate README when behavior spans layers.

## Planner maintenance
- Keep planning docs (`docs/roadmap.md`, `docs/implementation-plan.md`, `docs/tasks.md`) synchronized when behavior, ownership, or cadence changes.
- Record ownership only when assigned; prefer concrete names/teams over generic placeholders.

## For maintainers
If you update behavior in a crate, update:
1. The crate README (contract impact)
2. Related docs pages (user behavior)
3. Golden fixtures if output contracts changed
