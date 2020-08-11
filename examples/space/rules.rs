#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result};
use weasel::{
    battle_rules, battle_rules_with_space, rules::empty::*, BattleRules, Entities, Entity,
    EntityId, EventQueue, EventTrigger, PositionClaim, RemoveEntity, Rounds, SpaceRules,
    WeaselError, WeaselResult, WriteMetrics,
};

/// Length of each dimension of the battlefield.
const BATTLEFIELD_LENGTH: usize = 5;

// We define our own space rules.
#[derive(Default)]
pub(crate) struct CustomSpaceRules {}

impl SpaceRules<CustomRules> for CustomSpaceRules {
    // A square with two coordinates.
    type Position = Square;
    // The seed to initialize the battlefield.
    type SpaceSeed = BattlefieldSeed;
    // Our space model.
    type SpaceModel = Battlefield;
    // A vector containing the position of new traps.
    type SpaceAlteration = Vec<Square>;

    fn generate_model(&self, seed: &Option<Self::SpaceSeed>) -> Self::SpaceModel {
        Battlefield::from_seed(*seed)
    }

    fn check_move<'a>(
        &self,
        model: &Self::SpaceModel,
        _claim: PositionClaim<'a, CustomRules>,
        position: &Self::Position,
    ) -> WeaselResult<(), CustomRules> {
        // An entity can move into a square if it's free.
        if model.is_free(position) {
            Ok(())
        } else {
            Err(WeaselError::UserError("position occupied".to_string()))
        }
    }

    fn move_entity<'a>(
        &self,
        model: &mut Self::SpaceModel,
        claim: PositionClaim<'a, CustomRules>,
        position: Option<&Self::Position>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        if let Some(position) = position {
            // We simply insert the entity's id into a square of the model.
            match claim {
                PositionClaim::Spawn(id) => model.insert(position, *id),
                PositionClaim::Movement(entity) => model.insert(position, *entity.entity_id()),
            }
        } else {
            // Free the entity position.
            if let PositionClaim::Movement(entity) = claim {
                model.free(entity.position());
            }
        }
    }

    fn translate_entity(
        &self,
        _model: &Self::SpaceModel,
        new_model: &mut Self::SpaceModel,
        entity: &mut dyn Entity<CustomRules>,
        _event_queue: &mut Option<EventQueue<CustomRules>>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        // We are in completely new space.
        // Just take into consideration the x coordinate of an entity's position.
        new_model.insert(entity.position(), *entity.entity_id());
    }

    fn alter_space(
        &self,
        _entities: &Entities<CustomRules>,
        _rounds: &Rounds<CustomRules>,
        model: &mut Self::SpaceModel,
        alteration: &Self::SpaceAlteration,
        event_queue: &mut Option<EventQueue<CustomRules>>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        for trap_position in alteration {
            // Place a trap in the battlefield.
            model.place_trap(trap_position);
            // Remove any creature standing on a trap.
            if let Some(entity_id) = model.get(trap_position) {
                RemoveEntity::trigger(event_queue, entity_id).fire();
            }
        }
    }
}

battle_rules_with_space! { CustomSpaceRules }

/// Position for entities. It contains the coordinates of a square.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) struct Square {
    pub x: usize,
    pub y: usize,
}

/// Enum used as a seed to indicate which variant of the battlefield we want.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub(crate) enum BattlefieldSeed {
    OneDimension,
    TwoDimensions,
}

/// A struct containing an optional entity id and whether or not the square has a trap on it.
#[derive(Default, Clone, Copy)]
pub(crate) struct BattlefieldCell {
    entity: Option<EntityId<CustomRules>>,
    trap: bool,
}

/// The space model for this game.
#[allow(clippy::large_enum_variant)]
pub(crate) enum Battlefield {
    // Battlefield is empty a the start of the game.
    Empty,
    // One dimensional array of BattlefieldCell.
    OneDimension([BattlefieldCell; BATTLEFIELD_LENGTH]),
    // Two dimensional array of BattlefieldCell.
    TwoDimensions([[BattlefieldCell; BATTLEFIELD_LENGTH]; BATTLEFIELD_LENGTH]),
}

impl Battlefield {
    /// Creates a battlefield from a seed.
    fn from_seed(seed: Option<BattlefieldSeed>) -> Self {
        if let Some(seed) = seed {
            match seed {
                BattlefieldSeed::OneDimension => {
                    Self::OneDimension([BattlefieldCell::default(); BATTLEFIELD_LENGTH])
                }
                BattlefieldSeed::TwoDimensions => Self::TwoDimensions(
                    [[BattlefieldCell::default(); BATTLEFIELD_LENGTH]; BATTLEFIELD_LENGTH],
                ),
            }
        } else {
            Self::Empty
        }
    }

    /// Returns true if the given position is free.
    fn is_free(&self, position: &Square) -> bool {
        match self {
            Self::Empty => false,
            // For one dimensional battlefields we only care about the first dimension.
            Self::OneDimension(squares) => squares[position.x].entity.is_none(),
            Self::TwoDimensions(squares) => squares[position.y][position.x].entity.is_none(),
        }
    }

    /// Inserts the id of `entity` into `position`.
    fn insert(&mut self, position: &Square, entity: EntityId<CustomRules>) {
        match self {
            Self::Empty => {}
            Self::OneDimension(squares) => squares[position.x].entity = Some(entity),
            Self::TwoDimensions(squares) => squares[position.y][position.x].entity = Some(entity),
        }
    }

    /// Frees a position in the battlefield.
    fn free(&mut self, position: &Square) {
        match self {
            Self::Empty => {}
            Self::OneDimension(squares) => squares[position.x].entity = None,
            Self::TwoDimensions(squares) => squares[position.y][position.x].entity = None,
        }
    }

    /// Places a trap on the battlefield.
    fn place_trap(&mut self, position: &Square) {
        match self {
            Self::Empty => {}
            Self::OneDimension(squares) => squares[position.x].trap = true,
            Self::TwoDimensions(squares) => squares[position.y][position.x].trap = true,
        }
    }

    /// Get the entity on the given square.
    fn get(&self, position: &Square) -> Option<EntityId<CustomRules>> {
        match self {
            Self::Empty => None,
            Self::OneDimension(squares) => squares[position.x].entity,
            Self::TwoDimensions(squares) => squares[position.y][position.x].entity,
        }
    }
}

impl Display for Battlefield {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Empty => write!(f, "[]"),
            Self::OneDimension(squares) => {
                // Iterate over the array and print entities' ids and traps.
                for col in squares {
                    write!(f, "|")?;
                    if let Some(entity) = col.entity {
                        print_creature_id(f, entity)?;
                    } else if col.trap {
                        write!(f, "X")?;
                    } else {
                        write!(f, " ")?;
                    }
                }
                write!(f, "|")?;
                writeln!(f)?;
                Ok(())
            }
            Self::TwoDimensions(squares) => {
                // Iterate over the arrays and print entities' ids and traps.
                for (_, row) in squares.iter().rev().enumerate() {
                    for (_, col) in row.iter().enumerate() {
                        write!(f, "|")?;
                        if let Some(entity) = col.entity {
                            print_creature_id(f, entity)?;
                        } else if col.trap {
                            write!(f, "X")?;
                        } else {
                            write!(f, " ")?;
                        }
                    }
                    write!(f, "|")?;
                    writeln!(f)?;
                }
                Ok(())
            }
        }
    }
}

/// Function to print only the `CreatureId` part of an `EntityId`.
fn print_creature_id(f: &mut Formatter<'_>, id: EntityId<CustomRules>) -> Result {
    match id {
        EntityId::Creature(id) => write!(f, "{}", id),
        #[allow(unreachable_patterns)]
        _ => panic!("not expecting anything else than a creature"),
    }
}
