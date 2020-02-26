# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Example for undo/redo of events.
- Added a `GenericError` variant to `WeaselError`.
- Example to showcase passive abilities.

### Changed
- The methods `activable`, `on_round_start` and `on_round_end` now take `BattleState` as argument.
- The methods `allow_new_entity`, `activable`, `check_move` now return a `WeaselResult` instead of a bool.

## [0.4.1] - 2020-02-22
### Changed
- Replaced most usages of `HashMap` with `IndexMap`.

## [0.4.0] - 2020-02-21
### Added
- Doc tests for all events and few other structs.
- `Originated` decorator.
- Introduced inanimate objects.
- Improved public API for `Battle` and its submodules.

### Changed
- It's now possible to manually set an event's origin.
- New associated type `ObjectId` in `CharacterRules`.

## [0.3.1] - 2020-02-17
### Added
- Order of rounds and initiative example.
- Methods to retrieve an iterator over actors or characters.
- `on_actor_removed` method in `RoundsRules`.

## [0.3.0] - 2020-02-16
### Added
- `AlterSpace` event.
- Example showing different ways to manipulate the space model.

### Changed
- `SpaceRules`'s `check_move` and `move_entity` now take as argument a `PositionClaim` instead of an `Option<&dyn Entity<R>>`.
- `SpaceRules`'s `move_entity` is used also to move entities out of the space model.
- `RemoveCreature` frees the entity's position.
- `RoundsRules`'s and `on_start` and `on_end` take as arguments the entities and the space manager objects.

## [0.2.0] - 2020-02-15
### Added
- `RemoveTeam` event.
- An example showing how to use event sinks.
- Example to demonstrate how to create user defined events and metrics.
- `RegenerateStatistics` event.
- `RegenerateAbilities` event.
- `EntityId` now implements `Copy`.

## [0.1.0] - 2020-02-08
### Added
- First available version.
