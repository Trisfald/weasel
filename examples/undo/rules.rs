#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::fmt::{Display, Formatter, Result};
use weasel::rules::ability::SimpleAbility;
use weasel::{
    battle_rules, rules::empty::*, Action, ActorRules, BattleRules, BattleState, Entropy,
    EventQueue, EventTrigger, MoveEntity, PositionClaim, SpaceRules, WeaselError, WeaselResult,
    WriteMetrics,
};

/// Length of each dimension of the battlefield.
const BATTLEFIELD_LENGTH: usize = 5;

/// Id of the only ability in this game.
pub const WALK: u32 = 1;

// Use the `battle_rules` macro to quickly create an object that implements
// the `BattleRules` trait.
battle_rules! {
    // No special behavior for teams.
    EmptyTeamRules,
    // No need for creatures to have statistics.
    EmptyCharacterRules,
    CustomActorRules,
    // Creatures don't fight in this example.
    EmptyFightRules,
    // We don't use user defined metrics or events.
    EmptyUserRules,
    CustomSpaceRules,
    // We handle rounds manually.
    EmptyRoundsRules,
    // No randomness at all.
    EmptyEntropyRules
}

// We define our own space rules.
#[derive(Default)]
pub struct CustomSpaceRules {}

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

    fn check_move(
        &self,
        _model: &Self::SpaceModel,
        _claim: PositionClaim<CustomRules>,
        position: &Self::Position,
    ) -> WeaselResult<(), CustomRules> {
        // An entity can move into a square if it exists.
        // We don't check if the square is occupied because we know there will be only one entity.
        if position.valid() {
            Ok(())
        } else {
            Err(WeaselError::UserError(format!(
                "invalid position: {}",
                position
            )))
        }
    }

    fn move_entity(
        &self,
        model: &mut Self::SpaceModel,
        claim: PositionClaim<CustomRules>,
        position: Option<&Self::Position>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        if let Some(position) = position {
            match claim {
                PositionClaim::Spawn(_) => model.insert(*position),
                PositionClaim::Movement(entity) => model.change(*entity.position(), *position),
            }
        }
        // In this example the entity never leaves the battlefield, thus we don't care about the
        // else condition.
    }
}

/// Position for entities. It contains the coordinates of a square.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct Square {
    pub x: i8,
    pub y: i8,
}

impl Square {
    /// Returns another square which represents the same position after taking one step in
    /// the given direction.
    fn one_step_towards(self, dir: Direction) -> Self {
        match dir {
            Direction::Up => Self {
                x: self.x,
                y: self.y + 1,
            },
            Direction::Down => Self {
                x: self.x,
                y: self.y - 1,
            },
            Direction::Right => Self {
                x: self.x + 1,
                y: self.y,
            },
            Direction::Left => Self {
                x: self.x - 1,
                y: self.y,
            },
        }
    }

    fn valid(self) -> bool {
        let max = BATTLEFIELD_LENGTH.try_into().unwrap();
        let min = 0;
        self.x < max && self.x >= min && self.y < max && self.y >= min
    }
}

impl Display for Square {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "x: {}, y: {}", self.x, self.y)
    }
}

/// The space model for this game.
pub struct Battlefield {
    // A simple 2D battlefield. We only store if the square is occupied or not.
    squares: [[bool; BATTLEFIELD_LENGTH]; BATTLEFIELD_LENGTH],
}

impl Battlefield {
    /// Creates a battlefield.
    fn new() -> Self {
        Self {
            squares: [[false; BATTLEFIELD_LENGTH]; BATTLEFIELD_LENGTH],
        }
    }

    /// Marks one square as occupied.
    fn insert(&mut self, square: Square) {
        self.squares[square.y as usize][square.x as usize] = true;
    }

    /// Moves an entity from one square to another.
    fn change(&mut self, old: Square, new: Square) {
        self.squares[old.y as usize][old.x as usize] = false;
        self.insert(new);
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

// Define our custom actor rules.
#[derive(Default)]
pub struct CustomActorRules {}

impl ActorRules<CustomRules> for CustomActorRules {
    // Abilities will have fixed value.
    type Ability = SimpleAbility<u32, ()>;
    // No need for a seed. Same abilities for everyone.
    type AbilitiesSeed = ();
    // Our single ability needs to know the direction of movement.
    type Activation = Direction;
    // Abilities can't be altered in our game.
    type AbilitiesAlteration = ();

    fn generate_abilities(
        &self,
        _seed: &Option<Self::AbilitiesSeed>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Box<dyn Iterator<Item = Self::Ability>> {
        // We always generate a single ability, 'walk'.
        let v = vec![SimpleAbility::new(WALK, ())];
        Box::new(v.into_iter())
    }

    fn activable(
        &self,
        _state: &BattleState<CustomRules>,
        action: Action<CustomRules>,
    ) -> WeaselResult<(), CustomRules> {
        // The ability can be activated only if the destination exists.
        if let Some(dir) = action.activation {
            let destination = action.actor.position().one_step_towards(*dir);
            if destination.valid() {
                Ok(())
            } else {
                Err(WeaselError::UserError(format!(
                    "invalid destination: {}",
                    destination
                )))
            }
        } else {
            Err(WeaselError::UserError("missing activation!".to_string()))
        }
    }

    fn activate(
        &self,
        _state: &BattleState<CustomRules>,
        action: Action<CustomRules>,
        event_queue: &mut Option<EventQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        // To activate our only ability (walk) we just need to fire a MoveEntity event.
        let entity_id = *action.actor.entity_id();
        // Since this ability is activable, 'activation' will be set.
        let direction = action.activation.unwrap();
        // We also know that the new position is valid.
        let position = action.actor.position().one_step_towards(direction);
        MoveEntity::trigger(event_queue, entity_id, position).fire();
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub enum Direction {
    Up,
    Down,
    Right,
    Left,
}
