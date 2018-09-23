use uuid::Uuid;
use spatiub::spatial::Entity;
use std::time::UNIX_EPOCH;
use std::time::SystemTime;
use std::time::Duration;
use std::ops::Sub;
use std::fmt::Display;
use core::fmt;
use std::fmt::Formatter;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DemoEntity{
    pub id: Uuid,
    pub last_state_update: Timestamp,
}

impl Entity for DemoEntity{
    fn id(&self) -> &Uuid {
        &self.id
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Timestamp(Duration);

impl Timestamp{
    pub fn new() -> Timestamp{
        let start = SystemTime::now();
        let duration= start.duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        Timestamp(duration)
    }

    pub fn elapsed(&self) -> Duration {
        let current = Timestamp::new();
        current.0.sub(self.clone().0)
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let in_ms = self.0.as_secs() * 1000 + self.0.subsec_nanos() as u64 / 1_000_000;
        write!(f, "{}", in_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode;

    #[test]
    pub fn can_serialize_entity() {
        let entity = DemoEntity {
            id: Uuid::new_v4(),
            last_state_update: Timestamp::new(),
        };

        let serialized = bincode::serialize(&entity).unwrap();

        assert_eq!(entity, bincode::deserialize(&serialized).unwrap())
    }
}