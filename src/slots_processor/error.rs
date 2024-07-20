#[derive(Debug, thiserror::Error)]
pub enum SlotProcessingError {
    #[error(transparent)]
    ClientError(#[from] crate::clients::common::ClientError),
    #[error(transparent)]
    Provider(#[from] ethers::providers::ProviderError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    CustomError(String),
}

impl std::fmt::Display for SlotProcessingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{:?}", self) 
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SlotsProcessorError {
    #[error(
        "Error processing slots range {initial_slot}-{final_slot}. Slot {failed_slot} failed: {error}"
    )]
    FailedSlotsProcessing {
        initial_slot: u32,
        final_slot: u32,
        failed_slot: u32,
        error: SlotProcessingError,
    },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
