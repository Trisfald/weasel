use weasel::actor::{Action, ActorRules};
use weasel::battle::{BattleRules, BattleState};
use weasel::character::{
    AlterStatistics, Character, CharacterRules, StatisticId, StatisticsAlteration,
};
use weasel::entity::{EntityId, Transmutation};
use weasel::entropy::Entropy;
use weasel::event::{EventQueue, EventTrigger};
use weasel::fight::{ApplyImpact, FightRules};
use weasel::metric::{ReadMetrics, WriteMetrics};
use weasel::rules::entropy::UniformDistribution;
use weasel::rules::{ability::SimpleAbility, statistic::SimpleStatistic};
use weasel::team::{Conclusion, Team, TeamRules};
use weasel::util::Id;
use weasel::{battle_rules, rules::empty::*};

// Constants to identify statistics (of ships).
pub(crate) static STAT_HULL: StatisticId<PiratesRules> = 0;
pub(crate) static STAT_CREW: StatisticId<PiratesRules> = 1;
// Constants to identify abilities (of ships).
pub(crate) const ABILITY_CANNONBALL: &str = "cannonballs";
pub(crate) const ABILITY_GRAPESHOT: &str = "grapeshots";

// Define our custom team rules.
#[derive(Default)]
pub struct PiratesTeamRules {}

impl TeamRules<PiratesRules> for PiratesTeamRules {
    // We want to use a string as team id.
    type Id = String;
    // We we'll use the id of the opposing team both seed and objective, to check if the
    // goal of sinking the enemy ship was achieved.
    type ObjectivesSeed = Self::Id;
    type Objectives = Self::ObjectivesSeed;

    // Generate the objectives for a team. We said the seed and the objective are both
    // the enemy team id.
    fn generate_objectives(&self, seed: &Option<Self::ObjectivesSeed>) -> Self::Objectives {
        // So just unwrap the seed and return its content.
        seed.as_ref().unwrap().clone()
    }

    // We override the function to check objectives each time a round is finished.
    fn check_objectives_on_round(
        &self,
        state: &BattleState<PiratesRules>,
        team: &Team<PiratesRules>,
        _metrics: &ReadMetrics<PiratesRules>,
    ) -> Option<Conclusion> {
        // Get the objective of the team. Which is equal to its enemy id.
        let enemy_id = team.objectives();
        // Now check if the enemy has any creatures left.
        if state
            .entities()
            .team(&enemy_id)
            .unwrap()
            .creatures()
            .count()
            == 0
        {
            // We won.
            Some(Conclusion::Victory)
        } else {
            None
        }
    }
}

// Define our custom character rules.
#[derive(Default)]
pub struct PiratesCharacterRules {}

impl CharacterRules<PiratesRules> for PiratesCharacterRules {
    // We want an integer as creature id.
    type CreatureId = u8;
    // No inanimate objects in this game.
    type ObjectId = ();
    // Use statistics with integer as id and as value.
    type Statistic = SimpleStatistic<u8, i16>;
    // No need for a seed. All ships have the same statistics.
    type StatisticsSeed = ();
    // Our alteration for statistics consists of the values to add to HULL and to CREW.
    type StatisticsAlteration = (i16, i16);

    // In this method we generate statistics of ships.
    fn generate_statistics(
        &self,
        _seed: &Option<Self::StatisticsSeed>,
        _entropy: &mut Entropy<PiratesRules>,
        _metrics: &mut WriteMetrics<PiratesRules>,
    ) -> Box<dyn Iterator<Item = Self::Statistic>> {
        // Generate one statistic for the ship hull and another one for the ship crew.
        let v = vec![
            SimpleStatistic::new(STAT_HULL, 100),
            SimpleStatistic::new(STAT_CREW, 100),
        ];
        Box::new(v.into_iter())
    }

    // Method to alter the statistics of ships. In this case we want to decrease hull and crew.
    fn alter(
        &self,
        character: &mut dyn Character<PiratesRules>,
        alteration: &Self::StatisticsAlteration,
        _entropy: &mut Entropy<PiratesRules>,
        _metrics: &mut WriteMetrics<PiratesRules>,
    ) -> Option<Transmutation> {
        // As stated before our alteration contains the changes for both hull and crew.
        let (delta_hull, delta_crew) = alteration;
        character
            .statistic_mut(&STAT_HULL)
            .unwrap()
            .add(*delta_hull);
        character
            .statistic_mut(&STAT_CREW)
            .unwrap()
            .add(*delta_crew);
        // If hull reaches 0, the ships must be removed from the battle.
        if character.statistic(&STAT_HULL).unwrap().value() <= 0 {
            Some(Transmutation::REMOVAL)
        } else {
            None
        }
    }
}

// Define our custom actor rules.
#[derive(Default)]
pub struct PiratesActorRules {}

impl ActorRules<PiratesRules> for PiratesActorRules {
    // Use abilities with fixed power and string as id.
    type Ability = SimpleAbility<String, ()>;
    // No need for a seed. All ships have the same abilities.
    type AbilitiesSeed = ();
    // To activate an ability we will need to know who is the target.
    type Activation = EntityId<PiratesRules>;
    // Abilities can't be altered in our game.
    type AbilitiesAlteration = ();

    // In this method we generate abilities of ships.
    fn generate_abilities(
        &self,
        _seed: &Option<Self::AbilitiesSeed>,
        _entropy: &mut Entropy<PiratesRules>,
        _metrics: &mut WriteMetrics<PiratesRules>,
    ) -> Box<dyn Iterator<Item = Self::Ability>> {
        // We always generate two abilities for any ship.
        let v = vec![
            SimpleAbility::new(ABILITY_CANNONBALL.to_string(), ()),
            SimpleAbility::new(ABILITY_GRAPESHOT.to_string(), ()),
        ];
        Box::new(v.into_iter())
    }

    // Method called when an ability is activated.
    fn activate(
        &self,
        _state: &BattleState<PiratesRules>,
        action: Action<PiratesRules>,
        mut event_queue: &mut Option<EventQueue<PiratesRules>>,
        entropy: &mut Entropy<PiratesRules>,
        _metrics: &mut WriteMetrics<PiratesRules>,
    ) {
        // Retrieve the target from action, which is equal to activation.
        let target = action.activation.as_ref().unwrap();
        // We now compute the outcome of firing the cannons: an impact.
        // First retrieve the crew of this ship.
        let crew = action.actor.statistic(&STAT_CREW).unwrap().value();
        // Damage is 10 + a random value between crew/20 and crew/5.
        let damage = 10 + entropy.generate(crew / 20, crew / 5);
        // Damage hull if the ability is cannonball, otherwise the crew.
        let alteration = if action.ability.id() == ABILITY_CANNONBALL {
            (-damage, 0)
        } else {
            (0, -damage)
        };
        // Now we have everything we need to create an impact.
        // Triggers fired on the event_queue will spawn an event as soon as
        // the current one is done processing. The new events will have a link to the event who
        // generated them.
        ApplyImpact::trigger(&mut event_queue, (*target, alteration)).fire();
    }
}

// Define our custom fight rules.
#[derive(Default)]
pub struct PiratesFightRules {}

impl FightRules<PiratesRules> for PiratesFightRules {
    // Our impact type will be a tuple with target id and a statistics alteration.
    type Impact = (EntityId<PiratesRules>, StatisticsAlteration<PiratesRules>);

    fn apply_impact(
        &self,
        _state: &BattleState<PiratesRules>,
        impact: &Self::Impact,
        mut event_queue: &mut Option<EventQueue<PiratesRules>>,
        _entropy: &mut Entropy<PiratesRules>,
        _metrics: &mut WriteMetrics<PiratesRules>,
    ) {
        let target = &impact.0;
        // We know the target will be always there (we end the battle when the first ship sinks).
        // Thus we can safely create alteration events for the target.
        AlterStatistics::trigger(&mut event_queue, *target, impact.1).fire();
    }
}

// Finally, we use the `battle_rules` macro to quickly create an object that implements
// the `BattleRules` trait.
battle_rules! {
    PiratesTeamRules,
    PiratesCharacterRules,
    PiratesActorRules,
    PiratesFightRules,
    // We don't use user defined metrics or events.
    EmptyUserRules,
    // In our game ships don't move.
    EmptySpaceRules,
    // We handle rounds manually. The player always goes first.
    EmptyRoundsRules,
    UniformDistribution<i16>
}

pub(crate) type PiratesRules = CustomRules;
