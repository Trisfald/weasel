use weasel::rules::{statistic::SimpleStatistic, status::SimpleStatus};
use weasel::status::{Application, AppliedStatus, Potency, Status, StatusDuration, StatusId};
use weasel::{
    battle_rules, rules::empty::*, AlterStatistics, BattleRules, BattleState, Character,
    CharacterRules, Entropy, EventQueue, EventTrigger, FightRules, Id, LinkedQueue, Transmutation,
    WriteMetrics,
};

pub(crate) const HEALTH: u8 = 0;
/// Id of the status that increases HEALTH.
pub(crate) const VIGOR: u8 = 0;
/// Id of the status that deals damage over time.
pub(crate) const DOT: u8 = 1;

// Declare the battle rules with the help of a macro.
battle_rules! {
    EmptyTeamRules,
    // Use our own character rules to define how statuses are created.
    CustomCharacterRules,
    EmptyActorRules,
    // Use our own fight rules to specify the status' side effects.
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
    // This represents the quantity to add to the current value.
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
        let min = 0;
        let max = 100;
        let value = seed.unwrap();
        let v = vec![SimpleStatistic::with_value(HEALTH, min, max, value)];
        Box::new(v.into_iter())
    }

    fn alter_statistics(
        &self,
        character: &mut dyn Character<CustomRules>,
        alteration: &Self::StatisticsAlteration,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Option<Transmutation> {
        // Apply the change to the character's health.
        let current = character.statistic(&HEALTH).unwrap().value();
        character
            .statistic_mut(&HEALTH)
            .unwrap()
            .set_value(current + alteration);
        None
    }

    fn generate_status(
        &self,
        _character: &dyn Character<CustomRules>,
        status_id: &StatusId<CustomRules>,
        potency: &Option<Potency<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Option<Status<CustomRules>> {
        // We expect to always have a valid potency.
        let potency = potency.unwrap();
        let effect = potency.0;
        let duration = potency.1;
        // Return a new status in any case. If it already exists on the character,
        // the old one is replaced (anyway it doesn't happen in this example).
        Some(SimpleStatus::new(*status_id, effect, duration))
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
        // Treat all applications in the same way. We won't have replacements in this example.
        // So we only need to get the new status definition.
        let status = match application {
            Application::New(new) => new,
            Application::Replacement(_, new) => new,
        };
        // If the status is VIGOR buff the character's HEALTH.
        if *status.id() == VIGOR {
            AlterStatistics::trigger(event_queue, *character.entity_id(), status.effect()).fire();
        }
    }

    fn update_status(
        &self,
        _state: &BattleState<CustomRules>,
        character: &dyn Character<CustomRules>,
        status: &AppliedStatus<CustomRules>,
        linked_queue: &mut Option<LinkedQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> bool {
        // If the status is DOT deal some damage to the character.
        if *status.id() == DOT {
            AlterStatistics::trigger(linked_queue, *character.entity_id(), -status.effect()).fire();
        }
        // Terminate the status if its duration expired.
        if let Some(max_duration) = status.max_duration() {
            status.duration() == max_duration
        } else {
            false
        }
    }

    fn delete_status(
        &self,
        _state: &BattleState<CustomRules>,
        character: &dyn Character<CustomRules>,
        status: &AppliedStatus<CustomRules>,
        event_queue: &mut Option<EventQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        // If the status is VIGOR remove the buff.
        if *status.id() == VIGOR {
            AlterStatistics::trigger(event_queue, *character.entity_id(), -status.effect()).fire();
        }
    }
}
