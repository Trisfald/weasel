use std::fmt::{Display, Formatter, Result};
use weasel::actor::Actor;
use weasel::battle::{BattleRules, BattleState};
use weasel::character::{Character, CharacterRules, StatisticId};
use weasel::entity::{Entities, EntityId};
use weasel::entropy::Entropy;
use weasel::event::EventQueue;
use weasel::fight::FightRules;
use weasel::metric::WriteMetrics;
use weasel::rules::{statistic::SimpleStatistic, status::SimpleStatus};
use weasel::status::{Application, AppliedStatus, Potency, Status, StatusDuration, StatusId};
use weasel::{battle_rules, rules::empty::*};

pub(crate) static HEALTH: StatisticId<CustomRules> = 0;
/// Id of the status that deals damage over time.
pub(crate) static DOT: StatusId<CustomRules> = 0;
/// Id of the status that increases HEALTH.
pub(crate) static VIGOR: StatusId<CustomRules> = 0;

// Declare the battle rules with the help of a macro.
battle_rules! {
    EmptyTeamRules,
    // Use our own character rules to define how statuses are created.
    CustomCharacterRules,
    EmptyActorRules,
    // Use our own fight rules to specify what the status' side effects.
    CustomFightRules,
    EmptyUserRules,
    EmptySpaceRules,
    EmptyRoundsRules,
    EmptyEntropyRules
}

// Define our custom character rules.
#[derive(Default)]
pub struct CustomCharacterRules {}

impl CharacterRules<CustomRules> for CustomCharacterRules {
    // Just use an integer as creature id.
    type CreatureId = u8;
    // Same for objects.
    type ObjectId = u8;
    // Use statistics with integers as both id and value.
    type Statistic = SimpleStatistic<u8, i8>;
    // The seed will contain the value of HEALTH.
    type StatisticsSeed = i8;
    // We alter the HEALTH statistic in this example.
    // This represents how much we want to add to the current value.
    type StatisticsAlteration = i8;
    // Simple statuses with integers as both id and value.
    type Status = SimpleStatus<u8, i8>;
    // We don't alter statuses in this example.
    type StatusesAlteration = ();

    fn generate_statistics(
        &self,
        seed: &Option<Self::StatisticsSeed>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Box<dyn Iterator<Item = Self::Statistic>> {
        // Generate a single statistic: HEALTH.
        let v = vec![SimpleStatistic::new(HEALTH, seed.unwrap())];
        Box::new(v.into_iter())
    }

    fn generate_status(
        &self,
        character: &dyn Character<CustomRules>,
        status_id: &StatusId<CustomRules>,
        potency: &Option<Potency<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Option<Status<CustomRules>> {
        // TODO
        None
    }
}

// Define our custom fight rules.
#[derive(Default)]
pub struct CustomFightRules {}

impl FightRules<CustomRules> for CustomFightRules {
    // We don't use impacts.
    type Impact = ();
    // Potency will tell how strong a status is and how long will it lasts.
    type Potency = (i8, Option<StatusDuration>);

    fn apply_status(
        &self,
        _state: &BattleState<CustomRules>,
        character: &dyn Character<CustomRules>,
        application: Application<CustomRules>,
        event_queue: &mut Option<EventQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        // TODO
    }

    fn update_status(
        &self,
        _state: &BattleState<CustomRules>,
        character: &dyn Character<CustomRules>,
        status: &AppliedStatus<CustomRules>,
        event_queue: &mut Option<EventQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> bool {
        // TODO
        false
    }

    fn delete_status(
        &self,
        _state: &BattleState<CustomRules>,
        character: &dyn Character<CustomRules>,
        _status: &AppliedStatus<CustomRules>,
        event_queue: &mut Option<EventQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        // TODO
    }
}
