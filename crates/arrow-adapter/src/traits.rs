use crate::{
    contract::{AdapterInfo, AdapterInterchangeContract},
    receipt::{ConsumerExecutionMode, ConsumerReceipt},
};

/// Shared abstraction implemented by concrete origin adapters.
pub trait OriginAdapter {
    type Observation;
    type Error;

    fn info(&self) -> &AdapterInfo;
    fn observe(
        &self,
        observation: Self::Observation,
    ) -> Result<AdapterInterchangeContract, Self::Error>;
}

/// Shared abstraction implemented by concrete consumer adapters.
pub trait ConsumerAdapter {
    type Error;

    fn info(&self) -> &AdapterInfo;
    fn consume(
        &self,
        contract: &AdapterInterchangeContract,
        mode: ConsumerExecutionMode,
    ) -> Result<ConsumerReceipt, Self::Error>;
}
