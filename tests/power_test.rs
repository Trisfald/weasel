use weasel::battle::{BattleController, BattleRules, BattleState};
use weasel::entropy::Entropy;
use weasel::error::{WeaselError, WeaselResult};
use weasel::event::{EventQueue, EventTrigger};
use weasel::metric::WriteMetrics;
use weasel::power::{InvokePower, PowerId};
use weasel::rules::statistic::SimpleStatistic;
use weasel::team::{AlterPowers, Call, CreateTeam, RegeneratePowers, Team, TeamRules};
use weasel::{battle_rules, battle_rules_with_team, rules::empty::*, Id};

const TEAM_1_ID: u32 = 1;
const TEAM_ERR_ID: u32 = 99;

#[test]
fn powers_generated() {
    #[derive(Default)]
    pub struct CustomTeamRules {}

    impl<R: BattleRules + 'static> TeamRules<R> for CustomTeamRules {
        type Id = u32;
        type Power = EmptyPower;
        type PowersSeed = u32;
        type Invocation = ();
        type PowersAlteration = ();
        type ObjectivesSeed = ();
        type Objectives = ();

        fn generate_powers(
            &self,
            seed: &Option<Self::PowersSeed>,
            _entropy: &mut Entropy<R>,
            _metrics: &mut WriteMetrics<R>,
        ) -> Box<dyn Iterator<Item = Self::Power>> {
            if let Some(seed) = seed {
                let v = vec![EmptyPower { id: *seed }];
                Box::new(v.into_iter())
            } else {
                Box::new(std::iter::empty())
            }
        }
    }

    battle_rules_with_team! { CustomTeamRules }
    const SEED: u32 = 5;
    // Create a new team.
    let mut server = util::server(CustomRules::new());
    let mut trigger = CreateTeam::trigger(&mut server, TEAM_1_ID);
    let result = trigger.powers_seed(SEED).fire();
    assert_eq!(result.err(), None);
    // Check that powers are generated correctly.
    let team = server.battle().entities().team(&TEAM_1_ID).unwrap();
    let powers: Vec<_> = team.powers().collect();
    assert_eq!(powers, vec![&EmptyStat { id: SEED }]);
}

#[test]
fn alter_powers() {
    #[derive(Default)]
    pub struct CustomTeamRules {}

    impl TeamRules<CustomRules> for CustomTeamRules {
        type Id = u32;
        type Power = SimpleStatistic<u32, u32>;
        type PowersSeed = (u32, u32);
        type Invocation = ();
        type PowersAlteration = (u32, u32);
        type ObjectivesSeed = ();
        type Objectives = ();

        fn generate_powers(
            &self,
            seed: &Option<Self::PowersSeed>,
            _entropy: &mut Entropy<CustomRules>,
            _metrics: &mut WriteMetrics<CustomRules>,
        ) -> Box<dyn Iterator<Item = Self::Power>> {
            if let Some((id, value)) = seed {
                let v = vec![SimpleStatistic::new(*id, *value)];
                Box::new(v.into_iter())
            } else {
                Box::new(std::iter::empty())
            }
        }

        fn alter_powers(
            &self,
            team: &mut Team<CustomRules>,
            alteration: &Self::PowersAlteration,
            _entropy: &mut Entropy<CustomRules>,
            _metrics: &mut WriteMetrics<CustomRules>,
        ) {
            team.power_mut(&alteration.0)
                .unwrap()
                .set_value(alteration.1);
        }
    }

    battle_rules_with_team! { CustomTeamRules }

    static POWER_ID: PowerId<CustomRules> = 1;
    const POWER_VALUE: u32 = 10;
    const POWER_NEW_VALUE: u32 = 5;
    // Create a server with a team having one power.
    let mut server = util::server(CustomRules::new());
    assert_eq!(
        CreateTeam::trigger(&mut server, TEAM_1_ID)
            .powers_seed((POWER_ID, POWER_VALUE))
            .fire()
            .err(),
        None
    );
    // Powers alteration should fail for non existing teams.
    assert_eq!(
        AlterPowers::trigger(&mut server, TEAM_ERR_ID, (POWER_ID, 0))
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::TeamNotFound(TEAM_ERR_ID))
    );
    // Alter the team's powers.
    assert_eq!(
        AlterPowers::trigger(&mut server, TEAM_1_ID, (POWER_ID, POWER_NEW_VALUE))
            .fire()
            .err(),
        None
    );
    // Verify that the power's value has been changed.
    assert_eq!(
        server
            .battle()
            .entities()
            .team(&TEAM_1_ID)
            .unwrap()
            .power(&POWER_ID)
            .unwrap()
            .value(),
        POWER_NEW_VALUE
    );
}

#[test]
fn regenerate_powers() {
    #[derive(Default)]
    pub struct CustomTeamRules {}

    impl<R: BattleRules + 'static> TeamRules<R> for CustomTeamRules {
        type Id = u32;
        type Power = SimpleStatistic<u32, u32>;
        // Vec with pair (id, value).
        type PowersSeed = Vec<(u32, u32)>;
        type Invocation = ();
        type PowersAlteration = ();
        type ObjectivesSeed = ();
        type Objectives = ();

        fn generate_powers(
            &self,
            seed: &Option<Self::PowersSeed>,
            _entropy: &mut Entropy<R>,
            _metrics: &mut WriteMetrics<R>,
        ) -> Box<dyn Iterator<Item = Self::Power>> {
            if let Some(seed) = seed {
                let mut v = Vec::new();
                for (id, value) in seed {
                    v.push(SimpleStatistic::new(*id, *value));
                }
                Box::new(v.into_iter())
            } else {
                Box::new(std::iter::empty())
            }
        }
    }

    battle_rules_with_team! { CustomTeamRules }

    static POWER_1_ID: PowerId<CustomRules> = 1;
    static POWER_2_ID: PowerId<CustomRules> = 2;
    static POWER_3_ID: PowerId<CustomRules> = 3;
    const POWER_VALUE: u32 = 10;
    const POWER_ERR_VALUE: u32 = 0;
    // Create a new team with two powers.
    let mut server = util::server(CustomRules::new());
    assert_eq!(
        CreateTeam::trigger(&mut server, TEAM_1_ID)
            .powers_seed(vec![(POWER_1_ID, POWER_VALUE), (POWER_2_ID, POWER_VALUE)])
            .fire()
            .err(),
        None
    );
    assert_eq!(
        server
            .battle()
            .entities()
            .team(&TEAM_1_ID)
            .unwrap()
            .powers()
            .count(),
        2
    );
    // Regenerate should fail for non existing teams.
    assert_eq!(
        RegeneratePowers::trigger(&mut server, TEAM_ERR_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::TeamNotFound(TEAM_ERR_ID))
    );
    // Regenerate powers.
    assert_eq!(
        RegeneratePowers::trigger(&mut server, TEAM_1_ID)
            .seed(vec![
                (POWER_1_ID, POWER_ERR_VALUE),
                (POWER_3_ID, POWER_VALUE)
            ])
            .fire()
            .err(),
        None
    );
    let team = server.battle().entities().team(&TEAM_1_ID).unwrap();
    assert_eq!(team.powers().count(), 2);
    // Verify that one power was left untouched.
    assert_eq!(
        team.power(&POWER_1_ID),
        Some(&SimpleStatistic::new(POWER_1_ID, POWER_VALUE))
    );
    // Verify that one power was removed.
    assert!(team.power(&POWER_2_ID).is_none());
    // Verify that one power was added.
    assert_eq!(
        team.power(&POWER_3_ID),
        Some(&SimpleStatistic::new(POWER_3_ID, POWER_VALUE))
    );
}

#[test]
fn invoke_power() {
    #[derive(Default)]
    pub struct CustomTeamRules {}

    impl TeamRules<CustomRules> for CustomTeamRules {
        type Id = u32;
        type Power = EmptyPower;
        type PowersSeed = u32;
        type Invocation = ();
        type PowersAlteration = ();
        type ObjectivesSeed = ();
        type Objectives = ();

        fn generate_powers(
            &self,
            _seed: &Option<Self::PowersSeed>,
            _entropy: &mut Entropy<CustomRules>,
            _metrics: &mut WriteMetrics<CustomRules>,
        ) -> Box<dyn Iterator<Item = Self::Power>> {
            let v = vec![EmptyPower { id: POWER_1_ID }, EmptyPower { id: POWER_2_ID }];
            Box::new(v.into_iter())
        }

        fn invocable(
            &self,
            _state: &BattleState<CustomRules>,
            call: Call<CustomRules>,
        ) -> WeaselResult<(), CustomRules> {
            // Only the first power can be invoked.
            if *call.power.id() == POWER_1_ID {
                Ok(())
            } else {
                Err(WeaselError::GenericError)
            }
        }

        fn invoke(
            &self,
            _state: &BattleState<CustomRules>,
            _call: Call<CustomRules>,
            event_queue: &mut Option<EventQueue<CustomRules>>,
            _entropy: &mut Entropy<CustomRules>,
            _metrics: &mut WriteMetrics<CustomRules>,
        ) {
            // We trigger a dummy event to check if this method gets called.
            util::dummy(event_queue);
        }
    }

    battle_rules_with_team! { CustomTeamRules }

    static POWER_1_ID: PowerId<CustomRules> = 1;
    static POWER_2_ID: PowerId<CustomRules> = 2;
    static POWER_ERR_ID: PowerId<CustomRules> = 99;
    // Create a server with a team having two powers.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    // InvokePower should fail if the team doesn't exist.
    assert_eq!(
        InvokePower::trigger(&mut server, TEAM_ERR_ID, POWER_1_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::TeamNotFound(TEAM_ERR_ID))
    );
    // InvokePower should fail if the power is not known.
    assert_eq!(
        InvokePower::trigger(&mut server, TEAM_1_ID, POWER_ERR_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::PowerNotKnown(TEAM_1_ID, POWER_ERR_ID))
    );
    // InvokePower should fail if invocable returns false.
    assert_eq!(
        InvokePower::trigger(&mut server, TEAM_1_ID, POWER_2_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::PowerNotInvocable(
            TEAM_1_ID,
            POWER_2_ID,
            Box::new(WeaselError::GenericError)
        ))
    );
    // Fire a well defined event.
    // TODO
    // Verify that a dummy event was fired as a side effect of the power.
    // TODO
}
