use std::cell::RefCell;
use weasel::ability::ActivateAbility;
use weasel::actor::{Action, Actor, ActorRules};
use weasel::battle::{BattleRules, BattleState};
use weasel::battle_rules_with_team;
use weasel::creature::{ConvertCreature, CreateCreature, RemoveCreature};
use weasel::entity::EntityId;
use weasel::entropy::Entropy;
use weasel::event::{DummyEvent, EventKind, EventQueue, EventTrigger};
use weasel::metric::{system::*, ReadMetrics, WriteMetrics};
use weasel::player::PlayerId;
use weasel::team::{
    ConcludeObjectives, Conclusion, CreateTeam, EntityAddition, Relation, RemoveTeam,
    ResetObjectives, SetRelations, Team, TeamRules,
};
use weasel::{battle_rules, rules::empty::*, WeaselError, WeaselResult};

#[derive(Default)]
struct CustomTeamRules {
    allow_new_entities: RefCell<bool>,
    allow_converted_entities: RefCell<bool>,
}

impl<R: BattleRules> TeamRules<R> for CustomTeamRules {
    type Id = u32;
    type ObjectivesSeed = ();
    type Objectives = ();

    fn allow_new_entity(
        &self,
        _: &BattleState<R>,
        _: &Team<R>,
        mode: EntityAddition<R>,
    ) -> WeaselResult<(), R> {
        let allowed = match mode {
            EntityAddition::CreatureSpawn => *self.allow_new_entities.borrow(),
            EntityAddition::CreatureConversion(_) => *self.allow_converted_entities.borrow(),
        };
        if allowed {
            Ok(())
        } else {
            Err(WeaselError::GenericError)
        }
    }
}

const TEAM_1_ID: u32 = 1;
const TEAM_2_ID: u32 = 2;
const TEAM_3_ID: u32 = 3;
const TEAM_ERR_ID: u32 = 99;
const CREATURE_1_ID: u32 = 1;
const CREATURE_ERR_ID: u32 = 99;

#[test]
fn new_team() {
    battle_rules! {}
    // Check team creation.
    let mut server = util::server(CustomRules::new());
    for i in 0..2 {
        util::team(&mut server, i);
        assert!(server.battle().entities().team(&i).is_some());
    }
    // Check team duplication.
    let result = CreateTeam::trigger(&mut server, 0).fire();
    assert_eq!(
        result.err().map(|e| e.unfold()),
        Some(WeaselError::DuplicatedTeam(0))
    );
    assert_eq!(server.battle().entities().teams().count(), 2);
    // Check metrics.
    assert_eq!(server.battle().metrics().system_u64(TEAMS_CREATED), Some(2));
}

#[test]
fn creature_creation() {
    battle_rules_with_team! { CustomTeamRules }
    // Check that creature creation is taken into account.
    let mut rules = CustomRules::new();
    rules.team_rules = CustomTeamRules {
        allow_new_entities: RefCell::new(false),
        allow_converted_entities: RefCell::new(true),
    };
    let mut server = util::server(rules);
    util::team(&mut server, TEAM_1_ID);
    // Try to create a creature.
    let result = CreateCreature::trigger(&mut server, CREATURE_1_ID, TEAM_1_ID, ()).fire();
    assert_eq!(
        result.err().map(|e| e.unfold()),
        Some(WeaselError::NewCreatureUnaccepted(
            TEAM_1_ID,
            Box::new(WeaselError::GenericError)
        ))
    );
}

#[test]
fn diplomacy() {
    battle_rules! {}
    // Check team creation.
    let mut server = util::server(CustomRules::new());
    // Creating faulty diplomacy should fail.
    assert_eq!(
        CreateTeam::trigger(&mut server, TEAM_1_ID)
            .relations(&[(TEAM_2_ID, Relation::Ally)])
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::TeamNotFound(TEAM_2_ID))
    );
    // Create team one.
    util::team(&mut server, TEAM_1_ID);
    // Create team two allied with one.
    assert_eq!(
        CreateTeam::trigger(&mut server, TEAM_2_ID)
            .relations(&[(TEAM_1_ID, Relation::Ally)])
            .fire()
            .err(),
        None
    );
    // Check that kinship can't be set.
    assert_eq!(
        CreateTeam::trigger(&mut server, TEAM_3_ID)
            .relations(&[(TEAM_1_ID, Relation::Kin)])
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::KinshipRelation)
    );
    // Check self relation prevention.
    assert_eq!(
        CreateTeam::trigger(&mut server, TEAM_3_ID)
            .relations(&[(TEAM_2_ID, Relation::Ally), (TEAM_3_ID, Relation::Kin)])
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::SelfRelation)
    );
    // Create team three enemy with team one.
    assert_eq!(
        CreateTeam::trigger(&mut server, TEAM_3_ID)
            .relations(&[(TEAM_1_ID, Relation::Enemy)])
            .fire()
            .err(),
        None
    );
    // Check that diplomacy is created correctly.
    let entities = server.battle().entities();
    assert_eq!(entities.relation(&TEAM_1_ID, &TEAM_ERR_ID), None);
    assert_eq!(
        entities.relation(&TEAM_1_ID, &TEAM_1_ID),
        Some(Relation::Kin)
    );
    assert_eq!(
        entities.relation(&TEAM_1_ID, &TEAM_2_ID),
        Some(Relation::Ally)
    );
    assert_eq!(
        entities.relation(&TEAM_2_ID, &TEAM_1_ID),
        Some(Relation::Ally)
    );
    assert_eq!(
        entities.relation(&TEAM_1_ID, &TEAM_3_ID),
        Some(Relation::Enemy)
    );
    assert_eq!(
        entities.relation(&TEAM_3_ID, &TEAM_1_ID),
        Some(Relation::Enemy)
    );
    assert_eq!(
        entities.relation(&TEAM_3_ID, &TEAM_2_ID),
        Some(Relation::Enemy)
    );
    assert_eq!(
        entities.relation(&TEAM_2_ID, &TEAM_3_ID),
        Some(Relation::Enemy)
    );
    assert_eq!(
        entities.allies_id(&TEAM_1_ID).collect::<Vec<_>>(),
        vec![TEAM_2_ID]
    );
    assert_eq!(
        entities.enemies_id(&TEAM_1_ID).collect::<Vec<_>>(),
        vec![TEAM_3_ID]
    );
    assert_eq!(
        entities.allies_id(&TEAM_2_ID).collect::<Vec<_>>(),
        vec![TEAM_1_ID]
    );
    assert_eq!(
        entities.enemies_id(&TEAM_2_ID).collect::<Vec<_>>(),
        vec![TEAM_3_ID]
    );
    assert_eq!(
        entities.allies_id(&TEAM_3_ID).collect::<Vec<_>>(),
        vec![] as Vec<u32>
    );
    let mut vec = entities.enemies_id(&TEAM_3_ID).collect::<Vec<_>>();
    vec.sort_unstable();
    assert_eq!(vec, vec![TEAM_1_ID, TEAM_2_ID]);
}

#[test]
fn diplomacy_update() {
    battle_rules! {}
    // Create a battle with three teams.
    let mut server = util::server(CustomRules::new());
    // Create team one.
    util::team(&mut server, TEAM_1_ID);
    // Create team two allied with one.
    assert_eq!(
        CreateTeam::trigger(&mut server, TEAM_2_ID)
            .relations(&[(TEAM_1_ID, Relation::Ally)])
            .fire()
            .err(),
        None
    );
    // Create team three enemy with team one.
    assert_eq!(
        CreateTeam::trigger(&mut server, TEAM_3_ID)
            .relations(&[(TEAM_1_ID, Relation::Enemy)])
            .fire()
            .err(),
        None
    );
    // Set team three ally with team two and team one enemy with team two.
    // Check that pre-conditions are checked.
    assert_eq!(
        SetRelations::trigger(&mut server, &[(TEAM_1_ID, TEAM_ERR_ID, Relation::Ally)])
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::TeamNotFound(TEAM_ERR_ID))
    );
    assert_eq!(
        SetRelations::trigger(&mut server, &[(TEAM_ERR_ID, TEAM_1_ID, Relation::Ally)])
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::TeamNotFound(TEAM_ERR_ID))
    );
    assert_eq!(
        SetRelations::trigger(
            &mut server,
            &[
                (TEAM_1_ID, TEAM_2_ID, Relation::Ally),
                (TEAM_1_ID, TEAM_2_ID, Relation::Kin)
            ]
        )
        .fire()
        .err()
        .map(|e| e.unfold()),
        Some(WeaselError::KinshipRelation)
    );
    assert_eq!(
        SetRelations::trigger(
            &mut server,
            &[
                (TEAM_1_ID, TEAM_2_ID, Relation::Ally),
                (TEAM_1_ID, TEAM_1_ID, Relation::Ally)
            ]
        )
        .fire()
        .err()
        .map(|e| e.unfold()),
        Some(WeaselError::SelfRelation)
    );
    // Fire a correct event to change diplomacy.
    assert_eq!(
        SetRelations::trigger(
            &mut server,
            &[
                (TEAM_2_ID, TEAM_3_ID, Relation::Ally),
                (TEAM_1_ID, TEAM_2_ID, Relation::Enemy)
            ]
        )
        .fire()
        .err(),
        None
    );
    // Check that diplomacy is updated correctly.
    let entities = server.battle().entities();
    assert_eq!(
        entities.relation(&TEAM_1_ID, &TEAM_1_ID),
        Some(Relation::Kin)
    );
    assert_eq!(
        entities.allies_id(&TEAM_1_ID).collect::<Vec<_>>(),
        vec![] as Vec<u32>
    );
    let mut vec = entities.enemies_id(&TEAM_1_ID).collect::<Vec<_>>();
    vec.sort_unstable();
    assert_eq!(vec, vec![TEAM_2_ID, TEAM_3_ID]);
    assert_eq!(
        entities.allies_id(&TEAM_2_ID).collect::<Vec<_>>(),
        vec![TEAM_3_ID]
    );
    assert_eq!(
        entities.enemies_id(&TEAM_2_ID).collect::<Vec<_>>(),
        vec![TEAM_1_ID]
    );
    assert_eq!(
        entities.allies_id(&TEAM_3_ID).collect::<Vec<_>>(),
        vec![TEAM_2_ID]
    );
    assert_eq!(
        entities.enemies_id(&TEAM_3_ID).collect::<Vec<_>>(),
        vec![TEAM_1_ID]
    );
}

#[test]
fn convert_creature() {
    // Create a server with creature conversion disabled.
    battle_rules_with_team! { CustomTeamRules }
    let mut rules = CustomRules::new();
    rules.team_rules = CustomTeamRules {
        allow_new_entities: RefCell::new(true),
        allow_converted_entities: RefCell::new(false),
    };
    let mut server = util::server(rules);
    // Create two teams and one creature.
    util::team(&mut server, TEAM_1_ID);
    util::team(&mut server, TEAM_2_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    // Try faulty events.
    assert_eq!(
        ConvertCreature::trigger(&mut server, CREATURE_ERR_ID, TEAM_1_ID,)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::CreatureNotFound(CREATURE_ERR_ID))
    );
    assert_eq!(
        ConvertCreature::trigger(&mut server, CREATURE_1_ID, TEAM_ERR_ID,)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::TeamNotFound(TEAM_ERR_ID))
    );
    assert_eq!(
        ConvertCreature::trigger(&mut server, CREATURE_1_ID, TEAM_1_ID,)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::InvalidCreatureConversion(
            TEAM_1_ID,
            CREATURE_1_ID
        ))
    );
    assert_eq!(
        ConvertCreature::trigger(&mut server, CREATURE_1_ID, TEAM_2_ID,)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::ConvertedCreatureUnaccepted(
            TEAM_2_ID,
            CREATURE_1_ID,
            Box::new(WeaselError::GenericError)
        ))
    );
    // Check consistency.
    assert_eq!(
        *server
            .battle()
            .entities()
            .creature(&CREATURE_1_ID)
            .unwrap()
            .team_id(),
        TEAM_1_ID
    );
    let empty: [&u32; 0] = [];
    assert_eq!(
        *server
            .battle()
            .entities()
            .team(&TEAM_1_ID)
            .unwrap()
            .creatures()
            .collect::<Vec<_>>(),
        [&CREATURE_1_ID]
    );
    assert_eq!(
        *server
            .battle()
            .entities()
            .team(&TEAM_2_ID)
            .unwrap()
            .creatures()
            .collect::<Vec<_>>(),
        empty
    );
    // Enable creature conversion.
    *server
        .battle()
        .rules()
        .team_rules
        .allow_converted_entities
        .borrow_mut() = true;
    assert_eq!(
        ConvertCreature::trigger(&mut server, CREATURE_1_ID, TEAM_2_ID,)
            .fire()
            .err(),
        None
    );
    // Check consistency.
    assert_eq!(
        *server
            .battle()
            .entities()
            .creature(&CREATURE_1_ID)
            .unwrap()
            .team_id(),
        TEAM_2_ID
    );
    assert_eq!(
        *server
            .battle()
            .entities()
            .team(&TEAM_1_ID)
            .unwrap()
            .creatures()
            .collect::<Vec<_>>(),
        empty
    );
    assert_eq!(
        *server
            .battle()
            .entities()
            .team(&TEAM_2_ID)
            .unwrap()
            .creatures()
            .collect::<Vec<_>>(),
        [&CREATURE_1_ID]
    );
}

#[test]
fn conclusion() {
    battle_rules! {}
    let mut server = util::server(CustomRules::new());
    // Create two teams.
    util::team(&mut server, TEAM_1_ID);
    util::team(&mut server, TEAM_2_ID);
    // Check the teams state.
    assert_eq!(
        server
            .battle()
            .entities()
            .team(&TEAM_1_ID)
            .unwrap()
            .conclusion(),
        None
    );
    assert_eq!(
        server
            .battle()
            .entities()
            .team(&TEAM_2_ID)
            .unwrap()
            .conclusion(),
        None
    );
    assert_eq!(server.battle().entities().victorious().count(), 0);
    assert_eq!(server.battle().entities().defeated().count(), 0);
    // Make one team win and the other lose.
    // Check team existence.
    assert_eq!(
        ConcludeObjectives::trigger(&mut server, TEAM_ERR_ID, Conclusion::Victory)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::TeamNotFound(TEAM_ERR_ID))
    );
    assert_eq!(
        ConcludeObjectives::trigger(&mut server, TEAM_1_ID, Conclusion::Victory)
            .fire()
            .err(),
        None
    );
    assert_eq!(
        ConcludeObjectives::trigger(&mut server, TEAM_2_ID, Conclusion::Defeat)
            .fire()
            .err(),
        None
    );
    // Check the teams state.
    assert_eq!(
        server
            .battle()
            .entities()
            .team(&TEAM_1_ID)
            .unwrap()
            .conclusion(),
        Some(Conclusion::Victory)
    );
    assert_eq!(
        server
            .battle()
            .entities()
            .team(&TEAM_2_ID)
            .unwrap()
            .conclusion(),
        Some(Conclusion::Defeat)
    );
    assert_eq!(server.battle().entities().victorious().count(), 1);
    assert_eq!(server.battle().entities().defeated().count(), 1);
}

#[test]
fn reset_objectives() {
    #[derive(Default)]
    struct CustomTeamRules {}

    impl<R: BattleRules> TeamRules<R> for CustomTeamRules {
        type Id = u32;
        type ObjectivesSeed = u32;
        type Objectives = u32;

        fn generate_objectives(&self, seed: &Option<Self::ObjectivesSeed>) -> Self::Objectives {
            seed.unwrap_or_default()
        }
    }

    battle_rules_with_team! { CustomTeamRules }
    let mut server = util::server(CustomRules::new());
    // Team must exist.
    assert_eq!(
        ResetObjectives::trigger(&mut server, TEAM_ERR_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::TeamNotFound(TEAM_ERR_ID))
    );
    // Create a team.
    assert_eq!(
        CreateTeam::trigger(&mut server, TEAM_1_ID)
            .objectives_seed(5)
            .fire()
            .err(),
        None
    );
    // Make the team win.
    assert_eq!(
        ConcludeObjectives::trigger(&mut server, TEAM_1_ID, Conclusion::Victory)
            .fire()
            .err(),
        None
    );
    assert_eq!(
        *server
            .battle()
            .entities()
            .team(&TEAM_1_ID)
            .unwrap()
            .objectives(),
        5
    );
    assert_eq!(
        server
            .battle()
            .entities()
            .team(&TEAM_1_ID)
            .unwrap()
            .conclusion(),
        Some(Conclusion::Victory)
    );
    // Change its objectives.
    assert_eq!(
        ResetObjectives::trigger(&mut server, TEAM_1_ID)
            .seed(10)
            .fire()
            .err(),
        None
    );
    assert_eq!(
        *server
            .battle()
            .entities()
            .team(&TEAM_1_ID)
            .unwrap()
            .objectives(),
        10
    );
    assert_eq!(
        server
            .battle()
            .entities()
            .team(&TEAM_1_ID)
            .unwrap()
            .conclusion(),
        None
    );
}

#[test]
fn check_objectives() {
    #[derive(Default)]
    pub struct CustomActorRules {}

    impl ActorRules<CustomRules> for CustomActorRules {
        type Ability = EmptyAbility;
        type AbilitiesSeed = ();
        type Activation = ();
        type AbilitiesAlteration = ();

        fn generate_abilities(
            &self,
            _: &Option<Self::AbilitiesSeed>,
            _entropy: &mut Entropy<CustomRules>,
            _metrics: &mut WriteMetrics<CustomRules>,
        ) -> Box<dyn Iterator<Item = Self::Ability>> {
            let v = vec![EmptyAbility { id: ABILITY_ID }];
            Box::new(v.into_iter())
        }

        fn activate(
            &self,
            _state: &BattleState<CustomRules>,
            _action: Action<CustomRules>,
            mut event_queue: &mut Option<EventQueue<CustomRules>>,
            _entropy: &mut Entropy<CustomRules>,
            metrics: &mut WriteMetrics<CustomRules>,
        ) {
            DummyEvent::<_>::trigger(&mut event_queue).fire();
            metrics.add_user_u64(0, 1).unwrap();
        }
    }

    #[derive(Default)]
    struct CustomTeamRules {
        check_round: bool,
    }

    impl TeamRules<CustomRules> for CustomTeamRules {
        type Id = u32;
        type ObjectivesSeed = ();
        type Objectives = ();

        fn check_objectives_on_event(
            &self,
            _state: &BattleState<CustomRules>,
            _team: &Team<CustomRules>,
            metrics: &ReadMetrics<CustomRules>,
        ) -> Option<Conclusion> {
            if !self.check_round {
                if let Some(v) = metrics.user_u64(0) {
                    if v == 1 {
                        return Some(Conclusion::Victory);
                    }
                }
            }
            None
        }

        fn check_objectives_on_round(
            &self,
            _state: &BattleState<CustomRules>,
            _team: &Team<CustomRules>,
            metrics: &ReadMetrics<CustomRules>,
        ) -> Option<Conclusion> {
            if self.check_round {
                if let Some(v) = metrics.user_u64(0) {
                    if v == 1 {
                        return Some(Conclusion::Victory);
                    }
                }
            }
            None
        }
    }

    battle_rules! {
        CustomTeamRules,
        EmptyCharacterRules,
        CustomActorRules,
        EmptyFightRules,
        EmptyUserRules,
        EmptySpaceRules,
        EmptyRoundsRules,
        EmptyEntropyRules
    }

    const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
    const ABILITY_ID: u32 = 1;

    // Test round checks.
    // Create a battle with one creature.
    let mut rules = CustomRules::new();
    rules.team_rules = CustomTeamRules { check_round: true };
    let mut server = util::server(rules);
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    // Stard round and fire the ability.
    util::start_round(&mut server, &ENTITY_1_ID);
    assert_eq!(
        ActivateAbility::trigger(&mut server, ENTITY_1_ID, ABILITY_ID)
            .fire()
            .err(),
        None
    );
    // End round
    util::end_round(&mut server);
    // Victory should appear after the end round.
    assert_eq!(
        server
            .battle()
            .entities()
            .team(&TEAM_1_ID)
            .unwrap()
            .conclusion(),
        Some(Conclusion::Victory)
    );
    let events = server.battle().history().events();
    assert_eq!(
        events[events.len() - 1].kind(),
        EventKind::ConcludeObjectives
    );

    // Test event checks.
    // Create a battle with one creature.
    let mut rules = CustomRules::new();
    rules.team_rules = CustomTeamRules { check_round: false };
    let mut server = util::server(rules);
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    // Stard round and fire the ability.
    util::start_round(&mut server, &ENTITY_1_ID);
    assert_eq!(
        ActivateAbility::trigger(&mut server, ENTITY_1_ID, ABILITY_ID)
            .fire()
            .err(),
        None
    );
    // End round
    util::end_round(&mut server);
    // Victory should appear before the end round and the dummy event.
    assert_eq!(
        server
            .battle()
            .entities()
            .team(&TEAM_1_ID)
            .unwrap()
            .conclusion(),
        Some(Conclusion::Victory)
    );
    let events = server.battle().history().events();
    assert_eq!(
        events[events.len() - 3].kind(),
        EventKind::ConcludeObjectives
    );
    // Check we only have one ConcludeObjectives event.
    assert_eq!(
        events
            .iter()
            .filter(|event| event.kind() == EventKind::ConcludeObjectives)
            .count(),
        1
    );
}

#[test]
fn remove_team() {
    const PLAYER_1_ID: PlayerId = 1;
    battle_rules! {}
    // Create a battle with one team.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    // Add player rights to this team.
    assert_eq!(server.rights_mut().add(PLAYER_1_ID, &TEAM_1_ID).err(), None);
    assert!(server.rights().check(PLAYER_1_ID, &TEAM_1_ID));
    // Add a creature to the team.
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    assert_eq!(
        server
            .battle()
            .entities()
            .team(&TEAM_1_ID)
            .unwrap()
            .creatures()
            .count(),
        1
    );
    // Removing the team should fail if the id is invalid or the team is not empty.
    assert_eq!(
        RemoveTeam::trigger(&mut server, TEAM_ERR_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::TeamNotFound(TEAM_ERR_ID))
    );
    assert_eq!(
        RemoveTeam::trigger(&mut server, TEAM_1_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::TeamNotEmpty(TEAM_1_ID))
    );
    // Remove the creature and then the team.
    assert_eq!(
        RemoveCreature::trigger(&mut server, CREATURE_1_ID)
            .fire()
            .err(),
        None
    );
    assert_eq!(
        RemoveTeam::trigger(&mut server, TEAM_1_ID).fire().err(),
        None
    );
    // Check that both rights and team disappeared.
    assert!(!server.rights().check(PLAYER_1_ID, &TEAM_1_ID));
    assert!(server.battle().entities().team(&TEAM_1_ID).is_none());
}
