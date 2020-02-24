#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result};
use weasel::battle::BattleRules;
use weasel::metric::WriteMetrics;
use weasel::space::{PositionClaim, SpaceRules};
use weasel::{battle_rules, battle_rules_with_space, rules::empty::*};

/// Length of each dimension of the battlefield.
const BATTLEFIELD_LENGTH: usize = 5;

// We define our own space rules.
#[derive(Default)]
pub(crate) struct CustomSpaceRules {}

impl SpaceRules<CustomRules> for CustomSpaceRules {
    // A square with two coordinates.
    type Position = Square;
    // We always initialize the space in the same way, so no seed.
    type SpaceSeed = ();
    // Our space model.
    type SpaceModel = Battlefield;
    // In this example we don't alter the space.
    type SpaceAlteration = ();

    fn generate_model(&self, _seed: &Option<Self::SpaceSeed>) -> Self::SpaceModel {
        Battlefield::new()
    }

    fn check_move<'a>(
        &self,
        _model: &Self::SpaceModel,
        _claim: PositionClaim<'a, CustomRules>,
        position: &Self::Position,
    ) -> bool {
        // An entity can move into a square if it exists.
        // We don't check if the square is occupied because we know there will be only one entity.
        position.x < BATTLEFIELD_LENGTH && position.y < BATTLEFIELD_LENGTH
    }

    fn move_entity<'a>(
        &self,
        _model: &mut Self::SpaceModel,
        claim: PositionClaim<'a, CustomRules>,
        position: Option<&Self::Position>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        if let Some(_position) = position {
            match claim {
                PositionClaim::Spawn(_) => {}
                PositionClaim::Movement(_) => {}
            }
        }
        // In this example the entity never leaves the battlefield, thus we don't care about the
        // else condition.
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

/// The space model for this game.
pub(crate) struct Battlefield {
    // A simple 2D battlefield. We only store if the square is occupied or not.
    squares: [[bool; BATTLEFIELD_LENGTH]; BATTLEFIELD_LENGTH],
}

impl Battlefield {
    /// Creates a battlefield
    fn new() -> Battlefield {
        Battlefield {
            squares: [[false; BATTLEFIELD_LENGTH]; BATTLEFIELD_LENGTH],
        }
    }
}

impl Display for Battlefield {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // Iterate over the arrays and print the entity position.
        for (_, row) in self.squares.iter().rev().enumerate() {
            for (_, col) in row.iter().enumerate() {
                write!(f, "|")?;
                if *col {
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
