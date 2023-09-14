use async_trait::async_trait;

#[async_trait]
pub trait Publisher {
    async fn publish(&mut self, topic: String, data: Vec<u8>);
    // async fn destroy(&self);
}
