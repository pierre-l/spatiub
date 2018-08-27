use uuid::Uuid;
use spatiub::spatial::Entity;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DemoEntity{
    pub id: Uuid,
}

impl Entity for DemoEntity{
    fn id(&self) -> &Uuid {
        &self.id
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
        };

        let serialized = bincode::serialize(&entity).unwrap();

        assert_eq!(entity, bincode::deserialize(&serialized).unwrap())
    }
}