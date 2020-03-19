use indexmap::indexset;
#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use weasel::actor::Actor;
use weasel::battle::{Battle, BattleRules};
use weasel::entity::{Entities, EntityId};
use weasel::entropy::Entropy;
use weasel::event::{EventProcessor, EventRights, EventServer, EventTrigger};
use weasel::metric::{system::*, WriteMetrics};
use weasel::player::PlayerId;
use weasel::round::{EndRound, EnvironmentRound, ResetRounds, RoundState, RoundsRules, StartRound};
use weasel::server::Server;
use weasel::space::Space;
use weasel::WeaselError;
use weasel::{battle_rules, battle_rules_with_rounds, rules::empty::*};

const TEAM_1_ID: u32 = 1;
const TEAM_2_ID: u32 = 2;
const CREATURE_1_ID: u32 = 1;
const ENTITY_1_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_1_ID);
const CREATURE_2_ID: u32 = 2;
const ENTITY_2_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_2_ID);
const CREATURE_3_ID: u32 = 3;
const ENTITY_3_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_3_ID);
const CREATURE_ERR_ID: u32 = 99;
const ENTITY_ERR_ID: EntityId<CustomRules> = EntityId::Creature(CREATURE_ERR_ID);
const PLAYER_1_ID: PlayerId = 1;

#[derive(Clone, Default, Debug)]
#[cfg_attr(feature = "serialization", derive(Serialize, Deserialize))]
struct Model {
    starts: u32,
    ends: u32,
    adds: usize,
    last: Option<EntityId<CustomRules>>,
}

#[derive(Default)]
struct CustomRoundsRules {}

impl RoundsRules<CustomRules> for CustomRoundsRules {
    type RoundsSeed = Model;
    type RoundsModel = Model;

    fn generate_model(&self, seed: &Option<Self::RoundsSeed>) -> Self::RoundsModel {
        match seed {
            Some(seed) => seed.clone(),
            None => Model::default(),
        }
    }

    fn eligible(&self, model: &Self::RoundsModel, actor: &dyn Actor<CustomRules>) -> bool {
        // Entity 3 is always eligible.
        if *actor.entity_id() == ENTITY_3_ID {
            return true;
        }
        // Alterate rounds of entity 1 and 2.
        let entity_id = if model.last == Some(ENTITY_1_ID) {
            ENTITY_2_ID
        } else {
            ENTITY_1_ID
        };
        entity_id == *actor.entity_id()
    }

    fn on_start(
        &self,
        _entities: &Entities<CustomRules>,
        _space: &Space<CustomRules>,
        model: &mut Self::RoundsModel,
        actor: &dyn Actor<CustomRules>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        model.starts += 1;
        model.last = Some(*actor.entity_id());
    }

    fn on_end(
        &self,
        _entities: &Entities<CustomRules>,
        _space: &Space<CustomRules>,
        model: &mut Self::RoundsModel,
        _: &dyn Actor<CustomRules>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        model.ends += 1;
    }

    fn on_actor_added(
        &self,
        model: &mut Self::RoundsModel,
        _: &dyn Actor<CustomRules>,
        _entropy: &mut Entropy<CustomRules>,
        _metrics: &mut WriteMetrics<CustomRules>,
    ) {
        model.adds += 1;
    }
}

battle_rules_with_rounds! { CustomRoundsRules }

macro_rules! server {
    () => {{
        let mut model = Model::default();
        model.last = Some(ENTITY_2_ID);
        let battle = Battle::builder(CustomRules::new()).build();
        let mut server = Server::builder(battle).build();
        assert_eq!(
            ResetRounds::trigger(&mut server).seed(model).fire().err(),
            None
        );
        util::team(&mut server, TEAM_1_ID);
        util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
        util::creature(&mut server, CREATURE_2_ID, TEAM_1_ID, ());
        util::creature(&mut server, CREATURE_3_ID, TEAM_1_ID, ());
        server
    }};
}

#[test]
fn start_round() {
    // Initialize the battle.
    let mut server = server!();
    // Pre-start checks.
    assert_eq!(
        server.battle().rounds().model().adds,
        server.battle().entities().actors().count()
    );
    assert_eq!(*server.battle().rounds().state(), RoundState::<_>::Ready);
    // Check start round is prevented for faulty conditions.
    assert_eq!(
        StartRound::trigger(&mut server, ENTITY_ERR_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::EntityNotFound(ENTITY_ERR_ID))
    );
    assert_eq!(
        StartRound::trigger(&mut server, ENTITY_2_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::ActorNotEligible(ENTITY_2_ID))
    );
    assert_eq!(*server.battle().rounds().state(), RoundState::<_>::Ready);
    assert_eq!(server.battle().rounds().model().starts, 0);
    assert_eq!(server.battle().metrics().system_u64(ROUNDS_STARTED), None);
    // Check start works.
    util::start_round(&mut server, &ENTITY_1_ID);
    // Post-start checks.
    assert_eq!(
        *server.battle().rounds().state(),
        RoundState::<_>::Started(indexset! {ENTITY_1_ID})
    );
    assert_eq!(server.battle().rounds().model().starts, 1);
    assert_eq!(
        server.battle().metrics().system_u64(ROUNDS_STARTED),
        Some(1)
    );
    // Another start in a row must not work.
    assert_eq!(
        StartRound::trigger(&mut server, ENTITY_2_ID)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::RoundInProgress)
    );
    assert_eq!(
        *server.battle().rounds().state(),
        RoundState::<_>::Started(indexset! {ENTITY_1_ID})
    );
    assert_eq!(server.battle().rounds().model().starts, 1);
    assert_eq!(
        server.battle().metrics().system_u64(ROUNDS_STARTED),
        Some(1)
    );
}

#[test]
fn end_round() {
    // Initialize the battle.
    let mut server = server!();
    // Pre-start checks.
    assert_eq!(
        server.battle().rounds().model().adds,
        server.battle().entities().actors().count()
    );
    assert_eq!(*server.battle().rounds().state(), RoundState::<_>::Ready);
    // Check end round is prevented for faulty conditions.
    assert_eq!(
        EndRound::trigger(&mut server)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::NoRoundInProgress)
    );
    assert_eq!(server.battle().rounds().model().ends, 0);
    assert_eq!(*server.battle().rounds().state(), RoundState::<_>::Ready);
    // Start round.
    util::start_round(&mut server, &ENTITY_1_ID);
    // Check end works.
    util::end_round(&mut server);
    // Post-end checks.
    assert_eq!(server.battle().rounds().model().ends, 1);
    assert_eq!(*server.battle().rounds().state(), RoundState::<_>::Ready);
    // Check a new round can start.
    util::start_round(&mut server, &ENTITY_2_ID);
}

#[test]
fn reset_rounds() {
    // Initialize the battle.
    let mut server = server!();
    // Start a round.
    util::start_round(&mut server, &ENTITY_1_ID);
    // Check that rounds model can't be changed while the round is in progress.
    assert_eq!(
        ResetRounds::trigger(&mut server)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::RoundInProgress)
    );
    // Changing the rounds model in between rounds should be fine.
    util::end_round(&mut server);
    assert_eq!(ResetRounds::trigger(&mut server).fire().err(), None);
}

#[test]
fn environment_round() {
    // Initialize the battle.
    let mut server = server!();
    // Start a round.
    util::start_round(&mut server, &ENTITY_1_ID);
    // Check environment round is prevented for faulty conditions.
    assert_eq!(
        EnvironmentRound::trigger(&mut server)
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::RoundInProgress)
    );
    // End the current round and perform an enviroment round.
    util::end_round(&mut server);
    assert_eq!(EnvironmentRound::trigger(&mut server).fire().err(), None);
    // Verify metric increase.
    assert_eq!(
        server.battle().metrics().system_u64(ROUNDS_STARTED),
        Some(2)
    );
}

#[test]
fn round_multiple_actors() {
    // Initialize the battle.
    let mut server = server!();
    // Check that start round is prevented when at least one actor do not exist.
    assert_eq!(
        StartRound::trigger_with_actors(&mut server, vec![ENTITY_1_ID, ENTITY_ERR_ID])
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::EntityNotFound(ENTITY_ERR_ID))
    );
    // Check eligibility.
    assert_eq!(
        StartRound::trigger_with_actors(&mut server, vec![ENTITY_1_ID, ENTITY_2_ID])
            .fire()
            .err()
            .map(|e| e.unfold()),
        Some(WeaselError::ActorNotEligible(ENTITY_2_ID))
    );
    // Start a round.
    assert_eq!(
        StartRound::trigger_with_actors(&mut server, vec![ENTITY_1_ID, ENTITY_3_ID])
            .fire()
            .err(),
        None
    );
    // Post-start checks.
    assert_eq!(
        *server.battle().rounds().state(),
        RoundState::<_>::Started(indexset! {ENTITY_1_ID, ENTITY_3_ID})
    );
    assert_eq!(server.battle().rounds().model().starts, 2);
    assert_eq!(
        server.battle().metrics().system_u64(ROUNDS_STARTED),
        Some(1)
    );
    // End the round.
    util::end_round(&mut server);
    // Post-end checks.
    assert_eq!(server.battle().rounds().model().ends, 2);
    assert_eq!(*server.battle().rounds().state(), RoundState::<_>::Ready);
}

#[test]
fn round_actor_uniqueness() {
    // Initialize the battle.
    let mut server = server!();
    // Start a round with duplicated ids.
    assert_eq!(
        StartRound::trigger_with_actors(
            &mut server,
            vec![ENTITY_1_ID, ENTITY_3_ID, ENTITY_3_ID, ENTITY_1_ID]
        )
        .fire()
        .err(),
        None
    );
    // Verify that callbacks are invoked exactly once per actor.
    assert_eq!(server.battle().rounds().model().starts, 2);
}

#[test]
fn player_rights() {
    // Create a server with two creatures in different teams. Require authentication.
    let mut server = Server::builder(Battle::builder(CustomRules::new()).build())
        .enforce_authentication()
        .build();
    util::team(&mut server, TEAM_1_ID);
    util::team(&mut server, TEAM_2_ID);
    util::creature(&mut server, CREATURE_1_ID, TEAM_1_ID, ());
    util::creature(&mut server, CREATURE_3_ID, TEAM_2_ID, ());
    // Give to the player rights to only one team.
    assert_eq!(server.rights_mut().add(PLAYER_1_ID, &TEAM_2_ID).err(), None);
    // Verify rights for StartRound.
    let prototype =
        StartRound::trigger_with_actors(&mut server, vec![ENTITY_1_ID, ENTITY_3_ID]).prototype();
    let event = prototype.clone().client_prototype(0, Some(PLAYER_1_ID));
    assert_eq!(
        event.rights(server.battle()),
        EventRights::Teams(vec![&TEAM_1_ID, &TEAM_2_ID])
    );
    // StartRound should be blocked.
    assert_eq!(
        server.process_client(event).err().map(|e| e.unfold()),
        Some(WeaselError::AuthenticationError(
            Some(PLAYER_1_ID),
            TEAM_1_ID
        ))
    );
    // We need to start a real round in order to verify EndRound.
    // Bypass the rights checks by processing the event as a server.
    assert_eq!(server.process(prototype).err(), None);
    // Verify rights for EndRound.
    let event = EndRound::trigger(&mut server)
        .prototype()
        .client_prototype(0, Some(PLAYER_1_ID));
    assert_eq!(
        event.rights(server.battle()),
        EventRights::Teams(vec![&TEAM_1_ID, &TEAM_2_ID])
    );
    // EndRound should be blocked.
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
    // Check that now he can end the round.
    assert_eq!(server.process_client(event).err(), None);
}
