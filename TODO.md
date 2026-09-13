# TODO — Nightmare Shift

## Remaining AI work

- [ ] Expand the passenger roster beyond 16 with matching portraits, authored
  rules and dialogue, and semantic reference-coverage tests.
- [ ] Migrate the remaining `#[cfg(test)]` modules and test-only helpers from
  `src/` into `tests/` through intentional public APIs, preserving regression
  coverage and consolidating related cases toward five per major feature.
- [ ] Split the remaining oversized functions—especially
  `Game::handle_ui_action`, `Game::update`, and `draw_main_menu`—into focused
  helpers while keeping every function below the shared 100-line limit.
