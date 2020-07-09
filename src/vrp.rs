
pub struct Fabric {
    // A set of VR Actuator Blocks that are to be considered 1 unit
    pub name: String,
    pub uuid: uuid::Uuid,
}

impl Fabric {
    pub fn new(name: String) -> Fabric {
        Fabric {
            name,
            uuid: uuid::Uuid::new_v4(),
        }
    }
}