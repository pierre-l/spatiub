use entity::DemoEntity;
use spatiub::spatial::SpatialEvent;

#[derive(Serialize, Deserialize, Debug)]
pub enum Message{
    ConnectionAck(DemoEntity),
    Event(SpatialEvent<DemoEntity>)
}