use boxcars::{self, ActiveActor, Frame};
use std::{collections::HashMap, convert::TryFrom};

static BALL_TYPES: [&str; 5] = [
    "Archetypes.Ball.Ball_Default",
    "Archetypes.Ball.Ball_Basketball",
    "Archetypes.Ball.Ball_Puck",
    "Archetypes.Ball.CubeBall",
    "Archetypes.Ball.Ball_Breakout",
];

static BOOST_TYPE: &str = "Archetypes.CarComponents.CarComponent_Boost";
static JUMP_TYPE: &str = "Archetypes.CarComponents.CarComponent_Jump";
static DOUBLE_JUMP_TYPE: &str = "Archetypes.CarComponents.CarComponent_DoubleJump";
static DODGE_TYPE: &str = "Archetypes.CarComponents.CarComponent_Dodge";
static CAR_TYPE: &str = "Archetypes.Car.Car_Default";
static PLAYER_REPLICATION_KEY: &str = "Engine.Pawn:PlayerReplicationInfo";
static PLAYER_TYPE: &str = "TAGame.Default__PRI_TA";
static TEAM_TYPE: &str = "Engine.PlayerReplicationInfo:Team";
static GAME_TYPE: &str = "Archetypes.GameEvent.GameEvent_Soccar";

static BOOST_AMOUNT_KEY: &str = "TAGame.CarComponent_Boost_TA:ReplicatedBoostAmount";
static LAST_BOOST_AMOUNT_KEY: &str = "TAGame.CarComponent_Boost_TA:ReplicatedBoostAmount.Last";
static COMPONENT_ACTIVE_KEY: &str = "TAGame.CarComponent_TA:ReplicatedActive";
static RIGID_BODY_STATE_KEY: &str = "TAGame.RBActor_TA:ReplicatedRBState";
static UNIQUE_ID_KEY: &str = "Engine.PlayerReplicationInfo:UniqueId";
static VEHICLE_KEY: &str = "TAGame.CarComponent_TA:Vehicle";
static SECONDS_REMAINING_KEY: &str = "TAGame.GameEvent_Soccar_TA:SecondsRemaining";

static EMPTY_ACTOR_IDS: [boxcars::ActorId; 0] = [];

static BOOST_USED_PER_SECOND: f32 = 80.0 / 0.93;

#[derive(PartialEq, Debug, Clone)]
struct ActorState {
    attributes: HashMap<boxcars::ObjectId, boxcars::Attribute>,
    derived_attributes: HashMap<String, boxcars::Attribute>,
    object_id: boxcars::ObjectId,
    name_id: Option<i32>,
}

impl ActorState {
    fn new(new_actor: &boxcars::NewActor) -> Self {
        Self {
            attributes: HashMap::new(),
            derived_attributes: HashMap::new(),
            object_id: new_actor.object_id,
            name_id: new_actor.name_id,
        }
    }

    fn update_attribute(
        &mut self,
        update: &boxcars::UpdatedAttribute,
    ) -> Option<boxcars::Attribute> {
        self.attributes
            .insert(update.object_id, update.attribute.clone())
    }
}

struct ActorStateModeler {
    actor_states: HashMap<boxcars::ActorId, ActorState>,
    actor_ids_by_type: HashMap<boxcars::ObjectId, Vec<boxcars::ActorId>>,
}

impl ActorStateModeler {
    fn new() -> Self {
        Self {
            actor_states: HashMap::new(),
            actor_ids_by_type: HashMap::new(),
        }
    }

    fn process_frame(&mut self, frame: &boxcars::Frame) -> Result<(), String> {
        if let Some(err) = frame
            .deleted_actors
            .iter()
            .map(|n| self.delete_actor(n))
            .find(|r| r.is_err())
        {
            return err.map(|_| ());
        }
        if let Some(err) = frame
            .new_actors
            .iter()
            .map(|n| self.new_actor(n))
            .find(|r| r.is_err())
        {
            return err;
        }
        if let Some(err) = frame
            .updated_actors
            .iter()
            .map(|u| self.update_attribute(u))
            .find(|r| r.is_err())
        {
            return err.map(|_| ());
        }
        Ok(())
    }

    fn new_actor(&mut self, new_actor: &boxcars::NewActor) -> Result<(), String> {
        if let Some(state) = self.actor_states.get(&new_actor.actor_id) {
            if state.object_id != new_actor.object_id {
                return Err(format!(
                    "Tried to make new actor {:?}, existing state {:?}",
                    new_actor, state
                ));
            }
        } else {
            self.actor_states
                .insert(new_actor.actor_id, ActorState::new(new_actor));
            self.actor_ids_by_type
                .entry(new_actor.object_id)
                .or_insert_with(|| Vec::new())
                .push(new_actor.actor_id)
        }
        Ok(())
    }

    fn update_attribute(
        &mut self,
        update: &boxcars::UpdatedAttribute,
    ) -> Result<Option<boxcars::Attribute>, String> {
        self.actor_states
            .get_mut(&update.actor_id)
            .map(|state| state.update_attribute(update))
            .ok_or(format!(
                "Unable to find actor associated with update {:?}",
                update
            ))
    }

    fn delete_actor(&mut self, actor_id: &boxcars::ActorId) -> Result<ActorState, String> {
        let state = self
            .actor_states
            .remove(actor_id)
            .ok_or(format!("Unabled to delete actor id {:?}", actor_id))?;

        self.actor_ids_by_type
            .entry(state.object_id)
            .or_insert_with(|| Vec::new())
            .retain(|x| x != actor_id);

        Ok(state)
    }
}

type PlayerId = boxcars::UniqueId;

macro_rules! get_actor_attribute_matching {
    ($self:ident, $actor:expr, $prop:expr, $type:path) => {
        $self.get_actor_attribute($actor, $prop).and_then(|found| {
            attribute_match!(
                found,
                $type,
                format!(
                    "Actor {:?} value for {:?} not of the expected type",
                    $actor, $prop
                )
            )
        })
    };
}

macro_rules! attribute_match {
    ($value:expr, $type:path, $err:expr) => {
        if let $type(value) = $value {
            Ok(value)
        } else {
            Err($err)
        }
    };
}

macro_rules! get_attribute {
    ($self:ident, $map:expr, $prop:expr, $type:path) => {
        $self.get_attribute($map, $prop).and_then(|found| {
            attribute_match!(
                found,
                $type,
                format!("Value for {:?} not of the expected type, {:?}", $prop, $map)
            )
        })
    };
}

macro_rules! get_derived_attribute {
    ($map:expr, $key:expr, $type:path) => {
        $map.get($key)
            .ok_or(format!("No value for key: {:?}", $key))
            .and_then(|found| {
                attribute_match!(
                    found,
                    $type,
                    format!("Value for {:?} not of the expected type, {:?}", $key, $map)
                )
            })
    };
}

fn get_actor_id(active_actor: &ActiveActor) -> boxcars::ActorId {
    active_actor.actor
}

struct ReplayProcessor<'a> {
    replay: &'a boxcars::Replay,
    replay_data: ReplayData,
    actor_state: ActorStateModeler,
    object_id_to_name: HashMap<boxcars::ObjectId, String>,
    name_to_object_id: HashMap<String, boxcars::ObjectId>,
    ball_actor_id: Option<boxcars::ActorId>,
    player_to_actor_id: HashMap<PlayerId, boxcars::ActorId>,
    player_actor_to_car_actor: HashMap<boxcars::ActorId, boxcars::ActorId>,
    car_actor_to_boost_actor: HashMap<boxcars::ActorId, boxcars::ActorId>,
    car_actor_to_jump_actor: HashMap<boxcars::ActorId, boxcars::ActorId>,
    car_actor_to_double_jump_actor: HashMap<boxcars::ActorId, boxcars::ActorId>,
    car_actor_to_dodge_actor: HashMap<boxcars::ActorId, boxcars::ActorId>,
}

impl<'a> ReplayProcessor<'a> {
    fn new(replay: &'a boxcars::Replay) -> Self {
        let mut object_id_to_name = HashMap::new();
        let mut name_to_object_id = HashMap::new();
        for (id, name) in replay.objects.iter().enumerate() {
            let object_id = boxcars::ObjectId(id as i32);
            object_id_to_name.insert(object_id, name.clone());
            name_to_object_id.insert(name.clone(), object_id);
        }
        Self {
            actor_state: ActorStateModeler::new(),
            replay_data: ReplayData::new(),
            replay,
            object_id_to_name,
            name_to_object_id,
            ball_actor_id: None,
            player_actor_to_car_actor: HashMap::new(),
            player_to_actor_id: HashMap::new(),
            car_actor_to_boost_actor: HashMap::new(),
            car_actor_to_jump_actor: HashMap::new(),
            car_actor_to_double_jump_actor: HashMap::new(),
            car_actor_to_dodge_actor: HashMap::new(),
        }
    }

    fn get_data(mut self) -> Result<ReplayData, String> {
        for (index, frame) in self
            .replay
            .network_frames
            .as_ref()
            .unwrap()
            .frames
            .iter()
            .enumerate()
        {
            println!("{}", index);
            self.actor_state.process_frame(frame)?;
            self.update_player_to_car_mappings(frame)?;
            self.update_ball_id(frame)?;
            self.update_boost_amounts(frame)?;
            self.add_frame_to_replay_data(frame.time)?;
        }

        Ok(self.replay_data)
    }

    fn add_frame_to_replay_data(&mut self, time: f32) -> Result<(), String> {
        let metadata_frame = self.get_metadata_frame(time)?;
        let ball_frame = self.get_ball_frame()?;
        let player_frames = self.get_player_frames()?;
        self.replay_data
            .add_frame(metadata_frame, ball_frame, player_frames)?;
        Ok(())
    }

    fn get_metadata_frame(&self, time: f32) -> Result<MetadataFrame, String> {
        let actor_id = self
            .get_actor_ids_by_type(GAME_TYPE)
            .unwrap()
            .iter()
            .next()
            .ok_or("No game actor")?;
        let seconds_remaining = get_actor_attribute_matching!(
            self,
            actor_id,
            SECONDS_REMAINING_KEY,
            boxcars::Attribute::Int
        )?;
        println!("Seconds remaining: {:?}", seconds_remaining);
        Ok(MetadataFrame::new(
            time,
            u8::try_from(*seconds_remaining).map_err(|_| "Seconds remaining conversion failed")?,
        ))
    }

    fn get_object_id_for_key(&self, name: &str) -> Result<&boxcars::ObjectId, String> {
        self.name_to_object_id
            .get(name)
            .ok_or(format!("Could not get object id for name {:?}", name))
    }

    fn get_actor_ids_by_type(&self, name: &str) -> Result<&[boxcars::ActorId], String> {
        self.get_object_id_for_key(name)
            .map(|object_id| self.get_actor_ids_by_object_id(object_id))
    }

    fn get_actor_ids_vec(&self, name: &str) -> Result<Vec<boxcars::ActorId>, String> {
        Ok(self.get_actor_ids_by_type(name)?.iter().cloned().collect())
    }

    fn get_actor_ids_by_object_id(&self, object_id: &boxcars::ObjectId) -> &[boxcars::ActorId] {
        self.actor_state
            .actor_ids_by_type
            .get(object_id)
            .map(|v| &v[..])
            .unwrap_or_else(|| &EMPTY_ACTOR_IDS)
    }

    fn get_actor_state(
        &self,
        actor_id: &boxcars::ActorId,
    ) -> Result<&HashMap<boxcars::ObjectId, boxcars::Attribute>, String> {
        Ok(&self
            .actor_state
            .actor_states
            .get(actor_id)
            .ok_or(format!("Actor id, {:?} not found", actor_id))?
            .attributes)
    }

    fn get_actor_attribute<'b>(
        &'b self,
        actor_id: &boxcars::ActorId,
        property: &'b str,
    ) -> Result<&'b boxcars::Attribute, String> {
        self.get_attribute(self.get_actor_state(actor_id)?, property)
    }

    fn get_attribute<'b>(
        &'b self,
        map: &'b HashMap<boxcars::ObjectId, boxcars::Attribute>,
        property: &'b str,
    ) -> Result<&'b boxcars::Attribute, String> {
        let attribute_object_id = self
            .name_to_object_id
            .get(&property.to_string())
            .ok_or(format!("Could not find object_id for {:?}", property))?;
        map.get(attribute_object_id).ok_or(format!(
            "Could not find {:?} with object id {:?} on {:?}",
            property, attribute_object_id, map
        ))
    }

    fn find_ball_actor(&self) -> Option<boxcars::ActorId> {
        BALL_TYPES
            .iter()
            .filter_map(|ball_type| self.iter_actors_by_type(ball_type))
            .flat_map(|i| i)
            .map(|(actor_id, _)| actor_id.clone())
            .next()
    }

    fn update_ball_id(&mut self, frame: &boxcars::Frame) -> Result<(), String> {
        // XXX: This assumes there is only ever one ball, which is safe (I think?)
        if let Some(actor_id) = self.ball_actor_id {
            if frame.deleted_actors.contains(&actor_id) {
                self.ball_actor_id = None;
            }
        } else {
            self.ball_actor_id = self.find_ball_actor();
            if self.ball_actor_id.is_some() {
                return self.update_ball_id(frame);
            }
        }
        Ok(())
    }

    fn get_ball_frame(&self) -> Result<BallFrame, String> {
        if let Some(actor_id) = self.ball_actor_id {
            let rigid_body = get_actor_attribute_matching!(
                self,
                &actor_id,
                RIGID_BODY_STATE_KEY,
                boxcars::Attribute::RigidBody
            )?;
            Ok(BallFrame::from_data(rigid_body))
        } else {
            return Ok(BallFrame::Empty);
        }
    }

    fn update_player_to_car_mappings(&mut self, frame: &boxcars::Frame) -> Result<(), String> {
        for update in frame.updated_actors.iter() {
            macro_rules! maintain_actor_link {
                ($map:expr, $actor_type:expr, $attr:expr, $get_key: expr, $type:path) => {{
                    if &update.object_id == self.get_object_id_for_key(&$attr)? {
                        if self
                            .get_actor_ids_by_type($actor_type)?
                            .iter()
                            .any(|id| id == &update.actor_id)
                        {
                            let value = get_actor_attribute_matching!(
                                self,
                                &update.actor_id,
                                $attr,
                                $type
                            )?;
                            $map.insert($get_key(value), update.actor_id);
                        }
                    }
                }};
            }
            maintain_actor_link!(
                self.player_actor_to_car_actor,
                CAR_TYPE,
                PLAYER_REPLICATION_KEY,
                get_actor_id,
                boxcars::Attribute::ActiveActor
            );
            maintain_actor_link!(
                self.player_to_actor_id,
                PLAYER_TYPE,
                UNIQUE_ID_KEY,
                |unique_id: &Box<boxcars::UniqueId>| *unique_id.clone(),
                boxcars::Attribute::UniqueId
            );

            macro_rules! maintain_vehicle_key_link {
                ($map:expr, $actor_type:expr) => {
                    maintain_actor_link!(
                        $map,
                        $actor_type,
                        VEHICLE_KEY,
                        get_actor_id,
                        boxcars::Attribute::ActiveActor
                    )
                };
            }

            maintain_vehicle_key_link!(self.car_actor_to_boost_actor, BOOST_TYPE);
            maintain_vehicle_key_link!(self.car_actor_to_dodge_actor, DODGE_TYPE);
            maintain_vehicle_key_link!(self.car_actor_to_jump_actor, JUMP_TYPE);
            maintain_vehicle_key_link!(self.car_actor_to_double_jump_actor, DOUBLE_JUMP_TYPE);
        }

        for actor_id in frame.deleted_actors.iter() {
            self.player_actor_to_car_actor
                .remove(actor_id)
                .map(|car_id| {
                    println!("Player actor {:?} deleted, car id: {:?}.", actor_id, car_id)
                });
        }

        Ok(())
    }

    fn update_boost_amounts(&mut self, frame: &Frame) -> Result<(), String> {
        let updates: Vec<_> = self
            .iter_actors_by_type_err(BOOST_TYPE)?
            .map(|(actor_id, actor_state)| {
                let (actor_amount_value, last_value, _, derived_value, is_active) =
                    self.get_current_boost_values(actor_state);
                let mut current_value = if actor_amount_value == last_value {
                    // If we don't have an update in the actor, just continue using our derived value
                    derived_value
                } else {
                    // If we do have an update in the actor, use that value.
                    actor_amount_value.into()
                };
                if is_active {
                    current_value -= frame.delta * BOOST_USED_PER_SECOND;
                }
                (actor_id.clone(), current_value.max(0.0), actor_amount_value)
            })
            .collect();

        for (actor_id, current_value, new_last_value) in updates {
            let derived_attributes = &mut self
                .actor_state
                .actor_states
                .get_mut(&actor_id)
                .unwrap()
                .derived_attributes;

            derived_attributes.insert(
                LAST_BOOST_AMOUNT_KEY.to_string(),
                boxcars::Attribute::Byte(new_last_value),
            );
            derived_attributes.insert(
                BOOST_AMOUNT_KEY.to_string(),
                boxcars::Attribute::Float(current_value),
            );
        }
        Ok(())
    }

    fn get_current_boost_values(&self, actor_state: &ActorState) -> (u8, u8, u8, f32, bool) {
        let amount_value = get_attribute!(
            self,
            &actor_state.attributes,
            BOOST_AMOUNT_KEY,
            boxcars::Attribute::Byte
        )
        .cloned()
        .unwrap_or(0);
        let active_value = get_attribute!(
            self,
            &actor_state.attributes,
            COMPONENT_ACTIVE_KEY,
            boxcars::Attribute::Byte
        )
        .cloned()
        .unwrap_or(0);
        let is_active = active_value % 2 == 1;
        let derived_value = actor_state
            .derived_attributes
            .get(&BOOST_AMOUNT_KEY.to_string())
            .ok_or("No boost amount value.")
            .cloned()
            .and_then(|v| {
                attribute_match!(
                    v,
                    boxcars::Attribute::Float,
                    "Expected bool for derived value"
                )
            })
            .unwrap_or(0.0);
        let last_boost_amount = attribute_match!(
            actor_state
                .derived_attributes
                .get(&LAST_BOOST_AMOUNT_KEY.to_string())
                .cloned()
                .unwrap_or_else(|| boxcars::Attribute::Byte(amount_value)),
            boxcars::Attribute::Byte,
            "Expected byte value"
        )
        .unwrap_or(0);
        (
            amount_value,
            last_boost_amount,
            active_value,
            derived_value,
            is_active,
        )
    }

    fn get_car_actor(&self, player_id: &PlayerId) -> Result<&ActorState, String> {
        let car_actor_id = self.get_car_actor_id(player_id)?;
        self.actor_state
            .actor_states
            .get(&car_actor_id)
            .ok_or(format!("Car actor not found for id: {:?}", car_actor_id))
    }

    fn get_car_actor_id(&self, player_id: &PlayerId) -> Result<boxcars::ActorId, String> {
        let player_actor_id = self
            .player_to_actor_id
            .get(&player_id)
            .ok_or_else(|| format!("Could not find actor for player id {:?}", player_id))?;
        self.player_actor_to_car_actor
            .get(player_actor_id)
            .ok_or_else(|| format!("Car actor for player {:?} not known.", player_id))
            .cloned()
    }

    fn get_boost_actor_id(&self, player_id: &PlayerId) -> Result<boxcars::ActorId, String> {
        self.car_actor_to_boost_actor
            .get(&self.get_car_actor_id(player_id)?)
            .ok_or_else(|| format!("Boost actor for player {:?} not found", player_id))
            .cloned()
    }

    fn get_frame_for_player(&self, player_id: &PlayerId) -> Result<PlayerFrame, String> {
        let car_state = self.get_car_actor(player_id)?;
        let rigid_body = get_attribute!(
            self,
            &car_state.attributes,
            RIGID_BODY_STATE_KEY,
            boxcars::Attribute::RigidBody
        )?;
        let boost_state = self
            .actor_state
            .actor_states
            .get(&self.get_boost_actor_id(player_id)?)
            .ok_or(format!(
                "Could not find boost actor for player, {:?}",
                player_id
            ))?;
        let boost_amount = get_derived_attribute!(
            boost_state.derived_attributes,
            BOOST_AMOUNT_KEY,
            boxcars::Attribute::Float
        )?;

        println!("{:?}: {:?}", player_id, boost_amount * 100.0 / 255.0);
        Ok(PlayerFrame::from_data(rigid_body.clone(), *boost_amount))
    }

    fn get_player_frames(&self) -> Result<Vec<(PlayerId, PlayerFrame)>, String> {
        Ok(self
            .player_to_actor_id
            .keys()
            .map(|player_id| {
                (
                    player_id.clone(),
                    self.get_frame_for_player(player_id).unwrap_or_else(|e| {
                        println!("Error frame for {:?}, {}", player_id, e);
                        PlayerFrame::Empty
                    }),
                )
            })
            .collect())
    }

    fn map_attribute_keys(
        &self,
        hash_map: &HashMap<boxcars::ObjectId, boxcars::Attribute>,
    ) -> Result<HashMap<String, boxcars::Attribute>, ()> {
        hash_map
            .iter()
            .map(|(k, v)| {
                self.object_id_to_name
                    .get(k)
                    .map(|name| (name.clone(), v.clone()))
                    .ok_or(())
            })
            .collect()
    }

    fn iter_actors_by_type_err(
        &self,
        name: &str,
    ) -> Result<impl Iterator<Item = (&boxcars::ActorId, &ActorState)>, String> {
        self.iter_actors_by_type(name)
            .ok_or_else(|| format!("Couldn't find object id for {}", name))
    }

    fn iter_actors_by_type(
        &self,
        name: &str,
    ) -> Option<impl Iterator<Item = (&boxcars::ActorId, &ActorState)>> {
        self.name_to_object_id
            .get(name)
            .map(|id| self.iter_actors_by_object_id(id))
    }

    fn iter_actors_by_object_id<'b>(
        &'b self,
        object_id: &'b boxcars::ObjectId,
    ) -> impl Iterator<Item = (&'b boxcars::ActorId, &'b ActorState)> + 'b {
        let actor_ids = self
            .actor_state
            .actor_ids_by_type
            .get(object_id)
            .map(|v| &v[..])
            .unwrap_or_else(|| &EMPTY_ACTOR_IDS);

        actor_ids
            .iter()
            .map(move |id| (id, self.actor_state.actor_states.get(id).unwrap()))
    }

    fn actor_state_string(&self, actor_id: &boxcars::ActorId) -> String {
        format!(
            "{:?}",
            self.get_actor_state(actor_id)
                .map(|s| self.map_attribute_keys(s))
        )
    }

    fn print_actors_of_type(&self, actor_type: &str) {
        self.iter_actors_by_type(actor_type)
            .unwrap()
            .for_each(|(_actor_id, state)| {
                println!("{:?}", self.map_attribute_keys(&state.attributes));
            });
    }

    fn print_actor_types(&self) {
        let types: Vec<_> = self
            .actor_state
            .actor_ids_by_type
            .keys()
            .filter_map(|id| self.object_id_to_name.get(id))
            .collect();
        println!("{:?}", types);
    }
}

#[derive(Debug, Clone, PartialEq)]
enum BallFrame {
    Empty,
    Data { rigid_body: boxcars::RigidBody },
}

impl BallFrame {
    fn from_data(rigid_body: &boxcars::RigidBody) -> Self {
        Self::Data {
            rigid_body: rigid_body.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PlayerFrame {
    Empty,
    Data {
        rigid_body: boxcars::RigidBody,
        boost_amount: f32,
    },
}

impl PlayerFrame {
    fn from_data(rigid_body: boxcars::RigidBody, boost_amount: f32) -> Self {
        Self::Data {
            rigid_body,
            boost_amount,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PlayerData {
    frames: Vec<PlayerFrame>,
}

impl PlayerData {
    fn new() -> Self {
        Self { frames: Vec::new() }
    }

    fn add_frame(&mut self, frame_number: usize, frame: PlayerFrame) {
        let empty_frames_to_add = frame_number - self.frames.len();
        if empty_frames_to_add > 0 {
            for _ in 0..empty_frames_to_add {
                self.frames.push(PlayerFrame::Empty)
            }
        }
        self.frames.push(frame)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct BallData {
    frames: Vec<BallFrame>,
}

impl BallData {
    fn add_frame(&mut self, frame_number: usize, frame: BallFrame) {
        let empty_frames_to_add = frame_number - self.frames.len();
        if empty_frames_to_add > 0 {
            for _ in 0..empty_frames_to_add {
                self.frames.push(BallFrame::Empty)
            }
        }
        self.frames.push(frame)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct MetadataFrame {
    time: f32,
    seconds_remaining: u8,
}

impl MetadataFrame {
    fn new(time: f32, seconds_remaining: u8) -> Self {
        MetadataFrame {
            time,
            seconds_remaining,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ReplayData {
    ball_data: BallData,
    players: HashMap<PlayerId, PlayerData>,
    frame_metadata: Vec<MetadataFrame>,
}

impl ReplayData {
    fn new() -> Self {
        ReplayData {
            ball_data: BallData { frames: Vec::new() },
            players: HashMap::new(),
            frame_metadata: Vec::new(),
        }
    }

    fn add_frame(
        &mut self,
        frame_metadata: MetadataFrame,
        ball_frame: BallFrame,
        player_frames: Vec<(PlayerId, PlayerFrame)>,
    ) -> Result<(), String> {
        self.frame_metadata.push(frame_metadata);
        let frame_number = self.frame_metadata.len();
        self.ball_data.add_frame(frame_number, ball_frame);
        for (player_id, frame) in player_frames {
            self.players
                .entry(player_id)
                .or_insert_with(|| PlayerData::new())
                .add_frame(frame_number, frame)
        }
        Ok(())
    }
}

fn main() {
    let data = include_bytes!("../../aeda154d-a79c-490c-8c7f-0b8e9e43479d.replay");
    let parsing = boxcars::ParserBuilder::new(&data[..])
        .always_check_crc()
        .must_parse_network_data()
        .parse();
    let replay = parsing.unwrap();

    ReplayProcessor::new(&replay).get_data().unwrap();
}

// TODO: handle car sleeping
// TODO: Handle boost
// TODO: frame metadata
// TODO: Handle team assignment
// TODO: handle headers
// TODO: Handle jump
// TODO: TAGame.RBActor_TA:bIgnoreSyncing
// TODO: TAGame.GameEvent_Soccar_TA
