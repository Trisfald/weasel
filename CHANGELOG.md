# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Doc tests for all events and few other structs.
- `Originated` decorator to manually set the origin of an event.

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
