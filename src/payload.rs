use crate::ExecuteSignal;

#[async_trait::async_trait]
pub trait Payload: Send + Sync {
    async fn execute<T: ExecuteSignal>(signal: T) -> eyre::Result<()>;
}

#[derive(Clone, Debug)]
pub struct SendTx;

impl SendTx {
    pub fn new() -> Self {
        SendTx {}
    }
}

#[async_trait::async_trait]
impl Payload for SendTx {
    async fn execute<T: ExecuteSignal>(signal: T) -> eyre::Result<()> {
        // todo
        Ok(())
    }
}
