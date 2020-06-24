//! This module contains implementations for components that do nothing.
//! Such rules are useful if you don't need to implement any logic for a particular module.

use crate::actor::ActorRules;
use crate::battle::BattleRules;
use crate::character::CharacterRules;
use crate::fight::FightRules;
use crate::round::RoundsRules;
use crate::rules::entropy::FixedAverage;
use crate::space::SpaceRules;
use crate::team::TeamRules;
use crate::user::UserRules;
use crate::util::Id;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};

/// An empty statistic.
#[derive(Hash, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
pub struct EmptyStat {
    /// The id of this statistic.
    pub id: u32,
}

impl Id for EmptyStat {
    type Id = u32;
    fn id(&self) -> &u32 {
        &self.id
    }
}

/// An empty ability that does not contain any data.
pub type EmptyAbility = EmptyStat;

/// An empty status that does nothing.
pub type EmptyStatus = EmptyStat;

/// An empty power having no data nor behavior.
pub type EmptyPower = EmptyStat;

/// Minimalistic implementation of team rules, doing no-op for everything.
#[derive(Default)]
pub struct EmptyTeamRules {}

impl<R: BattleRules> TeamRules<R> for EmptyTeamRules {
    type Id = u32;
    type Power = EmptyPower;
    type PowerSeed = ();
    type ObjectivesSeed = ();
    type Objectives = ();
}

/// Minimalistic implementation of character rules, doing no-op for everything.
#[derive(Default)]
pub struct EmptyCharacterRules {}

impl<R: BattleRules> CharacterRules<R> for EmptyCharacterRules {
    type CreatureId = u32;
    type ObjectId = u32;
    type Statistic = EmptyStat;
    type StatisticsSeed = ();
    type StatisticsAlteration = ();
    type Status = EmptyStatus;
    type StatusesAlteration = ();
}

/// Minimalistic implementation of actor rules, doing no-op for everything.
#[derive(Default)]
pub struct EmptyActorRules {}

impl<R: BattleRules> ActorRules<R> for EmptyActorRules {
    type Ability = EmptyAbility;
    type AbilitiesSeed = ();
    type Activation = ();
    type AbilitiesAlteration = ();
}

/// Minimalistic implementation of space rules, doing no-op for everything.
#[derive(Default)]
pub struct EmptySpaceRules {}

impl<R: BattleRules> SpaceRules<R> for EmptySpaceRules {
    type Position = ();
    type SpaceSeed = ();
    type SpaceModel = ();
    type SpaceAlteration = ();

    fn generate_model(&self, _seed: &Option<Self::SpaceSeed>) -> Self::SpaceModel {}
}

/// Minimalistic implementation of rounds rules, doing no-op for everything.
#[derive(Default)]
pub struct EmptyRoundsRules {}

impl<R: BattleRules> RoundsRules<R> for EmptyRoundsRules {
    type RoundsSeed = ();
    type RoundsModel = ();

    fn generate_model(&self, _: &Option<Self::RoundsSeed>) -> Self::RoundsModel {}
}

/// Minimalistic implementation of fight rules, doing no-op for everything.
#[derive(Default)]
pub struct EmptyFightRules {}

impl<R: BattleRules> FightRules<R> for EmptyFightRules {
    type Impact = ();
    type Potency = ();
}

/// Minimalistic implementation of user rules, doing no-op for everything.
#[derive(Default)]
pub struct EmptyUserRules {}

impl<R: BattleRules> UserRules<R> for EmptyUserRules {
    type UserMetricId = u16;
    #[cfg(feature = "serialization")]
    type UserEventPackage = ();
}

/// Entropy rules that do not have randomness. They just return the average value.
pub type EmptyEntropyRules = FixedAverage<i32>;
