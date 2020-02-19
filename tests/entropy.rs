use weasel::actor::ActorRules;
use weasel::battle::{Battle, BattleRules};
use weasel::character::{Character, CharacterRules};
use weasel::entropy::{Entropy, ResetEntropy};
use weasel::event::EventTrigger;
use weasel::metric::WriteMetrics;
use weasel::rules::ability::SimpleAbility;
use weasel::rules::entropy::UniformDistribution;
use weasel::rules::statistic::SimpleStatistic;
use weasel::server::Server;
use weasel::{battle_rules, rules::empty::*};

#[cfg(feature = "serialization")]
mod helper;

static SEED: u64 = 1_204_678_643_940_597_513;
static TEAM_1_ID: u32 = 1;
static CREATURE_1_ID: u32 = 1;
static STAT_ID: u32 = 1;
static STAT_VALUE_MIN: i32 = 1;
static STAT_VALUE_MAX: i32 = 1000;
static STAT_VALUE: i32 = 820;
static ABILITY_ID: u32 = 1;
static ABILITY_POWER_MIN: i32 = 1;
static ABILITY_POWER_MAX: i32 = 1000;
static ABILITY_POWER: i32 = 33;

#[derive(Default)]
pub struct CustomCharacterRules {}

impl CharacterRules<CustomRules> for CustomCharacterRules {
    type CreatureId = u32;
    type Statistic = SimpleStatistic<u32, i32>;
    type StatisticsSeed = ();
    type StatisticsAlteration = ();

    fn generate_statistics(
        &self,
        _seed: &Option<Self::StatisticsSeed>,
        entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Box<dyn Iterator<Item = Self::Statistic>> {
        let value = entropy.generate(STAT_VALUE_MIN, STAT_VALUE_MAX);
        let v = vec![SimpleStatistic::new(STAT_ID, value)];
        Box::new(v.into_iter())
    }
}

#[derive(Default)]
pub struct CustomActorRules {}

impl ActorRules<CustomRules> for CustomActorRules {
    type Ability = SimpleAbility<u32, i32>;
    type AbilitiesSeed = ();
    type Activation = i32;
    type AbilitiesAlteration = ();

    fn generate_abilities(
        &self,
        _: &Option<Self::AbilitiesSeed>,
        entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) -> Box<dyn Iterator<Item = Self::Ability>> {
        let power = entropy.generate(ABILITY_POWER_MIN, ABILITY_POWER_MAX);
        let v = vec![SimpleAbility::new(ABILITY_ID, power)];
        Box::new(v.into_iter())
    }
}

battle_rules! {
    EmptyTeamRules,
    CustomCharacterRules,
    CustomActorRules,
    EmptyFightRules,
    EmptyUserRules,
    EmptySpaceRules,
    EmptyRoundsRules,
    UniformDistribution<i32>
}

/// Creates a scenario with a custom entropy model, one team and a creature.
macro_rules! scenario {
    () => {{
        // Create the battle.
        let battle = Battle::builder(CustomRules::new()).build();
        let mut server = Server::builder(battle).build();
        assert_eq!(
            ResetEntropy::trigger(&mut server).seed(SEED).fire().err(),
            None
        );
        // Create a team.
        util::team(&mut server, TEAM_1_ID);
        // Create a creature.
        util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
        server
    }};
}

/// Checks that statistics and abilities have been randomized as predicted.
macro_rules! stat_abi_randomness_check {
    ($server: expr) => {{
        let creature = $server.battle().entities().creature(&CREATURE_1_ID);
        assert!(creature.is_some());
        let creature = creature.unwrap();
        assert_eq!(creature.statistic(&STAT_ID).unwrap().value(), STAT_VALUE);
        assert_eq!(
            creature.ability(&ABILITY_ID).unwrap().power(),
            ABILITY_POWER
        );
    }};
}

#[test]
fn use_entropy() {
    let server = scenario!();
    // Check that statistics and abilities have been randomized.
    stat_abi_randomness_check!(server);
}

#[cfg(feature = "serialization")]
#[test]
fn entropy_reload() {
    let server = scenario!();
    // Check that statistics and abilities have been randomized.
    stat_abi_randomness_check!(server);
    // Save the battle.
    let history_json = helper::history_as_json(server.battle());
    // Restore the battle.
    let mut server = util::server(CustomRules::new());
    helper::load_json_history(&mut server, history_json);
    // Verify that randomization is the same.
    stat_abi_randomness_check!(server);
}
