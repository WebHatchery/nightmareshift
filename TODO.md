# TODO — Nightmare Shift

## Standards and correctness

- [ ] Expose testable game logic through `src/lib.rs`, make `main.rs` use it, and migrate all tests and test-only helpers from `src/` into `tests/` through intentional public APIs (§11.4). Preserve regression coverage; consolidate related cases toward five per major feature and explain any necessary excess (§11.3).
- [ ] Split cohesive responsibilities out of `src/ui/core.rs`, `src/state/game_state.rs`, `src/engine/ride_service/route_choice.rs`, `src/game.rs`, and `src/game/rules.rs` before they grow beyond 800 physical lines. Migrate affected legacy `mod.rs` roots to named module files, add missing module-purpose docs, and correct the outdated “non-test lines” comment in `tests/code_standards.rs` (§2.2–2.3, §9.2).
- [ ] Break functions exceeding 100 lines into focused helpers, starting with `Game::handle_ui_action`, `Game::update`, `draw_main_menu`, and `draw_ui_icon`. Replace broad argument-count suppression in `src/main.rs` with cohesive parameter structs or narrowly documented exceptions (§4, §10.2).
- [ ] Rewrite handbook steps and shortcut-only prompts to name exact visible controls; update `game_page.json` and README controls to describe touch paths and the actual D-to-decline binding (§7.5).
- [ ] Verify and fix narrow touch-browser layouts, especially the fixed-width seed dialog and stacked Help/Options panels. Exercise a complete first fare, scrolling, settings, and recovery using taps only at portrait and landscape sizes (§7.5).

## Remaining features and content

- [ ] Add versioned save migration with fixtures for supported older schemas; retain protection against unsupported newer saves.
- [ ] Implement mid-run save/resume, including RNG state, campaign state, and simulation timers.
- [ ] Add validated save export/import with visible browser controls and recoverable import errors.
- [ ] Add configurable key bindings with conflict handling and matching shortcut labels; add gamepad navigation and gameplay mappings through shared toolkit input support where practical.
- [ ] Map authored passenger/guideline audio cues to distinct sounds instead of routing unknown cue names to the generic warning; add UI feedback sounds through the existing mixer and caption settings.
- [ ] Expand the passenger roster beyond 16 with matching portraits, authored rules/dialogue, and validated references.
- [ ] Add location-specific distance/fuel costs, spawn affinities, and destination-leg risk to JSON schemas and route calculations.
- [ ] Integrate the existing unused logo, driving-background, and item art from `assets/` into appropriate screens; verify portrait-to-character correspondence and add restrained reaction animation that respects reduced motion.
- [ ] Add headlight and route-darkness effects tied to cab controls and route state.
- [ ] Add a credits sequence with contributor and asset attribution.
- [ ] Introduce localized narrative keys, a language selector, and one non-English locale; replace ASCII stripping with font coverage for the chosen script.

## Automation

- [ ] Add a deterministic playtest-bot smoke gate to the centrally maintained CI workflow in `rust_management`, with virtual-display/software-GL setup, timeout handling, and failure artifacts; distribute it through the shared sync workflow.
