

#[allow(dead_code)]
#[derive(Debug)]
pub struct Fabric {
    // A set of VR Actuator Blocks that are to be considered 1 unit
    pub name: String,
    pub uuid: uuid::Uuid,
}

#[allow(dead_code)]
impl Fabric {
    pub fn new(name: &str) -> Fabric {
        Fabric {
            name: String::from(name),
            uuid: uuid::Uuid::new_v4(),
        }
    }
}