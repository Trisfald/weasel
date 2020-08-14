use weasel::ability::ActivateAbility;
use weasel::actor::{Action, ActorRules};
use weasel::battle::{Battle, BattleController, BattleRules, BattleState};
use weasel::entity::EntityId;
use weasel::entropy::Entropy;
use weasel::event::{DummyEvent, EventKind, EventQueue, EventRights, EventServer, EventTrigger};
use weasel::metric::WriteMetrics;
use weasel::player::PlayerId;
use weasel::rules::empty::EmptyAbility;
use weasel::{
    battle_rules, battle_rules_with_actor, rules::empty::*, Server, WeaselError, WeaselResult,
};

const TEAM_1_ID: u32 = 1;
const TEAM_2_ID: u32 = 2;
const CREATURE_1_ID: u32 = 1;
const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
const CREATURE_ERR_ID: u32 = 5;
const ENTITY_ERR_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_ERR_ID);
const ABILITY_ID: u32 = 1;
const ABILITY_ERR_ID: u32 = 5;
const PLAYER_1_ID: PlayerId = 1;

#[derive(Default)]
pub struct CustomActorRules {}

impl ActorRules<CustomRules> for CustomActorRules {
    type Ability = EmptyAbility;
    type AbilitiesSeed = u32;
    type Activation = u32;
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

    fn activable(
        &self,
        _state: &BattleState<CustomRules>,
        action: Action<CustomRules>,
    ) -> WeaselResult<(), CustomRules> {
        if action.activation.is_some() {
            Ok(())
        } else {
            Err(WeaselError::GenericError)
        }
    }

    fn activate(
        &self,
        _state: &BattleState<CustomRules>,
        action: Action<CustomRules>,
        mut event_queue: &mut Option<EventQueue<CustomRules>>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        let count = action.activation.unwrap();
        for _ in 0..count {
            DummyEvent::trigger(&mut event_queue).fire();
        }
    }
}

battle_rules_with_actor! { CustomActorRules }

#[test]
fn abilities_generated() {
    // Create a server with a creature.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    // Verify that abilities were generated.
    assert_eq!(
        server
            .battle()
            .entities()
            .actor(&ENTITY_1_ID)
            .unwrap()
            .abilities()
            .count(),
        1
    );
}

#[test]
fn ability_activation() {
    // Create a server with a creature.
    let mut server = util::server(CustomRules::new());
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    // Ability done by a missing creature should fail.
    assert_eq!(
        ActivateAbility::trigger(&mut server, ENTITY_ERR_ID, ABILITY_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::EntityNotFound(ENTITY_ERR_ID))
    );
    // Fail when creature has not started the turn.
    assert_eq!(
        ActivateAbility::trigger(&mut server, ENTITY_1_ID, ABILITY_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::ActorNotReady(ENTITY_1_ID))
    );
    // Start a turn.
    util::start_turn(&mut server, &ENTITY_1_ID);
    // Fail when creature does not know the ability.
    assert_eq!(
        ActivateAbility::trigger(&mut server, ENTITY_1_ID, ABILITY_ERR_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::AbilityNotKnown(ENTITY_1_ID, ABILITY_ERR_ID))
    );
    // Fail when `activable` returns false.
    assert_eq!(
        ActivateAbility::trigger(&mut server, ENTITY_1_ID, ABILITY_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::AbilityNotActivable(
            ENTITY_1_ID,
            ABILITY_ID,
            Box::new(WeaselError::GenericError)
        ))
    );
    // Succeed in activating an ability.
    assert_eq!(
        ActivateAbility::trigger(&mut server, ENTITY_1_ID, ABILITY_ID)
            .activation(2)
            .fire()
            .err(),
        None
    );
    let events = server.battle().history().events();
    assert!(events.len() >= 2);
    assert_eq!(events[events.len() - 2].kind(), EventKind::DummyEvent);
    assert_eq!(events[events.len() - 1].kind(), EventKind::DummyEvent);
    assert_eq!(events[events.len() - 2].origin(), Some(3));
    assert_eq!(events[events.len() - 1].origin(), Some(3));
}

#[test]
fn ability_rights() {
    // Create a server with a creature. Require authentication.
    let mut server = Server::builder(Battle::builder(CustomRules::new()).build())
        .enforce_authentication()
        .build();
    util::team(&mut server, TEAM_1_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    // Create another team.
    util::team(&mut server, TEAM_2_ID);
    // Give to the player rights to the team without any creature.
    assert_eq!(server.rights_mut().add(PLAYER_1_ID, &TEAM_2_ID).err(), None);
    // Check event rights.
    util::start_turn(&mut server, &ENTITY_1_ID);
    let event = ActivateAbility::trigger(&mut server, ENTITY_1_ID, ABILITY_ID)
        .activation(2)
        .prototype()
        .client_prototype(0, Some(PLAYER_1_ID));
    assert_eq!(
        event.event().rights(server.battle()),
        EventRights::Team(&TEAM_1_ID)
    );
    // Ability activation should be rejected.
    assert_eq!(
        server
            .process_client(event.clone())
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::AuthenticationError(
            Some(PLAYER_1_ID),
            TEAM_1_ID
        ))
    );
    // Give rights to the player.
    assert_eq!(server.rights_mut().add(PLAYER_1_ID, &TEAM_1_ID).err(), None);
    // Check that now he can activate the ability.
    assert_eq!(server.process_client(event).err(), None);
}
