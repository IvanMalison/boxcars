use bitter::BitGet;
use attributes::Attribute;

/// An object's current vector
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Vector {
    pub bias: i32,
    pub dx: i32,
    pub dy: i32,
    pub dz: i32,
}

impl Vector {
    pub fn decode(bits: &mut BitGet) -> Option<Vector> {
        if_chain! {
            if let Some(size_bits) = bits.read_bits_max(5, 20);
            let bias = 1 << (size_bits + 1);
            let bit_limit = (size_bits + 2) as i32;
            if let Some(dx) = bits.read_u32_bits(bit_limit);
            if let Some(dy) = bits.read_u32_bits(bit_limit);
            if let Some(dz) = bits.read_u32_bits(bit_limit);
            then {
                Some(Vector {
                    bias: bias as i32,
                    dx: dx as i32,
                    dy: dy as i32,
                    dz: dz as i32,
                })
            } else {
                None
            }
        }
    }

    pub fn decode_unchecked(bits: &mut BitGet) -> Vector {
        let size_bits = bits.read_bits_max_unchecked(5, 20);
        let bias = 1 << (size_bits + 1);
        let bit_limit = (size_bits + 2) as i32;
        let dx = bits.read_u32_bits_unchecked(bit_limit);
        let dy = bits.read_u32_bits_unchecked(bit_limit);
        let dz = bits.read_u32_bits_unchecked(bit_limit);
        Vector {
            bias: bias as i32,
            dx: dx as i32,
            dy: dy as i32,
            dz: dz as i32,
        }
    }
}

/// An object's current rotation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Rotation {
    pub yaw: Option<i8>,
    pub pitch: Option<i8>,
    pub roll: Option<i8>,
}

impl Rotation {
    pub fn decode(bits: &mut BitGet) -> Option<Rotation> {
        if_chain! {
            if let Some(yaw) = bits.if_get(|b| b.read_i8());
            if let Some(pitch) = bits.if_get(|b| b.read_i8());
            if let Some(roll) = bits.if_get(|b| b.read_i8());
            then {
                Some(Rotation {
                    yaw: yaw,
                    pitch: pitch,
                    roll: roll,
                })
            } else {
                None
            }
        }
    }

    pub fn decode_unchecked(bits: &mut BitGet) -> Rotation {
        let yaw = bits.if_get_unchecked(|b| b.read_i8_unchecked());
        let pitch = bits.if_get_unchecked(|b| b.read_i8_unchecked());
        let roll = bits.if_get_unchecked(|b| b.read_i8_unchecked());
        Rotation {
            yaw: yaw,
            pitch: pitch,
            roll: roll,
        }
    }
}

/// When a new actor spawns in rocket league it will either have a location, location and rotation,
/// or none of the above
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnTrajectory {
    None,
    Location,
    LocationAndRotation,
}

/// Notifies that an actor has had one of their properties updated (most likely their rigid body
/// state (location / rotation) has changed)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UpdatedAttribute {
    /// The actor that had an attribute updated
    pub actor_id: i32,

    /// The attribute / property id that was decoded
    pub attribute_id: i32,

    /// The actual data from the decoded attribute
    pub attribute: Attribute,
}

/// Contains the time and any new information that occurred during a frame
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Frame {
    /// The time in seconds that the frame is recorded at
    pub time: f32,

    /// Time difference between previous frame
    pub delta: f32,

    /// List of new actors seen during the frame
    pub new_actors: Vec<NewActor>,

    /// List of actor id's that are deleted / destroyed
    pub deleted_actors: Vec<i32>,

    /// List of properties updated on the actors
    pub updated_actors: Vec<UpdatedAttribute>,
}

/// Information for a new actor that appears in the game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct NewActor {
    /// The id given to the new actor
    pub actor_id: i32,

    /// An name id
    pub name_id: Option<i32>,

    /// The actor's type. Is an index to to `Replay::objects`
    pub object_ind: i32,

    /// The initial trajectory of the new actor
    pub initial_trajectory: Trajectory,
}

/// Contains the optional location and rotation of an object when it spawns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Trajectory {
    pub location: Option<Vector>,
    pub rotation: Option<Rotation>,
}

impl Trajectory {
    pub fn from_spawn(bits: &mut BitGet, sp: SpawnTrajectory) -> Option<Trajectory> {
        match sp {
            SpawnTrajectory::None => Some(Trajectory {
                location: None,
                rotation: None,
            }),

            SpawnTrajectory::Location => Vector::decode(bits).map(|v| Trajectory {
                location: Some(v),
                rotation: None,
            }),

            SpawnTrajectory::LocationAndRotation => if_chain! {
                if let Some(v) = Vector::decode(bits);
                if let Some(r) = Rotation::decode(bits);
                then {
                    Some(Trajectory {
                        location: Some(v),
                        rotation: Some(r),
                    })
                } else {
                    None
                }
            },
        }
    }

    pub fn from_spawn_unchecked(bits: &mut BitGet, sp: SpawnTrajectory) -> Trajectory {
        match sp {
            SpawnTrajectory::None => Trajectory {
                location: None,
                rotation: None,
            },

            SpawnTrajectory::Location => Trajectory {
                location: Some(Vector::decode_unchecked(bits)),
                rotation: None,
            },

            SpawnTrajectory::LocationAndRotation => Trajectory {
                location: Some(Vector::decode_unchecked(bits)),
                rotation: Some(Rotation::decode_unchecked(bits)),
            },
        }
    }
}

/// Oftentimes a replay contains many different objects of the same type. For instance, each rumble
/// pickup item is of the same type but has a different name. The name of:
/// `stadium_foggy_p.TheWorld:PersistentLevel.VehiclePickup_Boost_TA_30` should be normalized to
/// `TheWorld:PersistentLevel.VehiclePickup_Boost_TA` so that we don't have to work around each
/// stadium and pickup that is released.
pub fn normalize_object(name: &str) -> &str {
    if name.contains("TheWorld:PersistentLevel.CrowdActor_TA") {
        "TheWorld:PersistentLevel.CrowdActor_TA"
    } else if name.contains("TheWorld:PersistentLevel.CrowdManager_TA") {
        "TheWorld:PersistentLevel.CrowdManager_TA"
    } else if name.contains("TheWorld:PersistentLevel.VehiclePickup_Boost_TA") {
        "TheWorld:PersistentLevel.VehiclePickup_Boost_TA"
    } else if name.contains("TheWorld:PersistentLevel.InMapScoreboard_TA") {
        "TheWorld:PersistentLevel.InMapScoreboard_TA"
    } else if name.contains("TheWorld:PersistentLevel.BreakOutActor_Platform_TA") {
        "TheWorld:PersistentLevel.BreakOutActor_Platform_TA"
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_vector() {
        let mut bitter = BitGet::new(&[0b0000_0110, 0b0000_1000, 0b1101_1000, 0b0000_1101]);
        let v = Vector::decode(&mut bitter).unwrap();
        assert_eq!(
            v,
            Vector {
                bias: 128,
                dx: 128,
                dy: 128,
                dz: 221,
            }
        );
    }

    #[test]
    fn test_decode_vector_unchecked() {
        let mut bitter = BitGet::new(&[0b0000_0110, 0b0000_1000, 0b1101_1000, 0b0000_1101]);
        let v = Vector::decode_unchecked(&mut bitter);
        assert_eq!(
            v,
            Vector {
                bias: 128,
                dx: 128,
                dy: 128,
                dz: 221,
            }
        );
    }

    #[test]
    fn test_decode_rotation() {
        let mut bitter = BitGet::new(&[0b0000_0101, 0b0000_0000]);
        let v = Rotation::decode(&mut bitter).unwrap();
        assert_eq!(
            v,
            Rotation {
                yaw: Some(2),
                pitch: None,
                roll: None,
            }
        );
    }

    #[test]
    fn test_decode_rotation_unchecked() {
        let mut bitter = BitGet::new(&[0b0000_0101, 0b0000_0000]);
        let v = Rotation::decode_unchecked(&mut bitter);
        assert_eq!(
            v,
            Rotation {
                yaw: Some(2),
                pitch: None,
                roll: None,
            }
        );
    }
}
