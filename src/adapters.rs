#![allow(async_fn_in_trait)]

pub mod aggregators;
pub mod prop;
pub mod public;

pub trait Adapter {
    async fn quote(&self) -> eyre::Result<()>;
    async fn swap(&self) -> eyre::Result<()>;
}

pub trait Aggregator: Adapter {}
pub trait Prop: Adapter {}
pub trait Public: Adapter {}
