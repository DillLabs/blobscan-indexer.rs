use anyhow::{anyhow, Context as AnyhowContext, Result};

use ethers::prelude::*;
use tracing::{debug, info};

use crate::{
    clients::{
        beacon::types::BlockId,
        blobscan::types::{Blob, Block, Transaction},
    },
    context::Context,
};

use self::error::{SlotProcessorError, SlotsProcessorError};
use self::helpers::{create_tx_hash_versioned_hashes_mapping, create_versioned_hash_blob_mapping};

pub mod error;
mod helpers;

pub struct SlotProcessor {
    context: Context,
}

impl SlotProcessor {
    pub fn new(context: Context) -> SlotProcessor {
        Self { context }
    }

    pub async fn process_slots(
        &self,
        from_slot: u32,
        to_slot: u32,
    ) -> Result<(), SlotsProcessorError> {
        for current_slot in from_slot..to_slot {
            let result = self.process_slot(current_slot).await;

            if let Err(error) = result {
                return Err(SlotsProcessorError::FailedSlotsRange {
                    initial_slot: from_slot,
                    final_slot: to_slot,
                    failed_slot: current_slot,
                    error,
                });
            }
        }

        Ok(())
    }

    pub async fn process_slot(&self, slot: u32) -> Result<(), SlotProcessorError> {
        let beacon_client = self.context.beacon_client();
        let blobscan_client = self.context.blobscan_client();
        let provider = self.context.provider();

        // Fetch execution block data from a given slot and perform some checks

        let beacon_block = match beacon_client
            .get_block(&BlockId::Slot(slot))
            .await
            .map_err(SlotProcessorError::ClientError)?
        {
            Some(block) => block,
            None => {
                debug!(
                    target = "slot_processor",
                    slot, "Skipping as there is no beacon block"
                );

                return Ok(());
            }
        };

        let execution_payload = match beacon_block.message.body.execution_payload {
            Some(payload) => payload,
            None => {
                debug!(
                    target = "slot_processor",
                    slot, "Skipping as beacon block doesn't contain execution payload"
                );

                return Ok(());
            }
        };

        let has_kzg_blob_commitments = match beacon_block.message.body.blob_kzg_commitments {
            Some(commitments) => !commitments.is_empty(),
            None => false,
        };

        if !has_kzg_blob_commitments {
            debug!(
                target = "slot_processor",
                slot, "Skipping as beacon block doesn't contain blob kzg commitments"
            );

            return Ok(());
        }

        let execution_block_hash = execution_payload.block_hash;

        // Fetch execution block and perform some checks

        let execution_block = provider
            .get_block_with_txs(execution_block_hash)
            .await?
            .with_context(|| format!("Execution block {execution_block_hash} not found"))?;

        let tx_hash_to_versioned_hashes =
            create_tx_hash_versioned_hashes_mapping(&execution_block)?;

        if tx_hash_to_versioned_hashes.is_empty() {
            return Err(anyhow!("Blocks mismatch: Beacon block contains blob KZG commitments, but the corresponding execution block does not contain any blob transactions").into());
        }

        // Fetch blobs and perform some checks

        let blobs = match beacon_client
            .get_blobs(&BlockId::Slot(slot))
            .await
            .map_err(SlotProcessorError::ClientError)?
        {
            Some(blobs) => {
                if blobs.is_empty() {
                    debug!(
                        target = "slot_processor",
                        slot, "Skipping as blobs sidecar is empty"
                    );

                    return Ok(());
                } else {
                    blobs
                }
            }
            None => {
                debug!(
                    target = "slot_processor",
                    slot, "Skipping as there is no blobs sidecar"
                );

                return Ok(());
            }
        };

        // Create entities to be indexed

        let block_entity = Block::try_from((&execution_block, slot))?;

        let transactions_entities = execution_block
            .transactions
            .iter()
            .filter(|tx| tx_hash_to_versioned_hashes.contains_key(&tx.hash))
            .map(|tx| Transaction::try_from((tx, &execution_block)))
            .collect::<Result<Vec<Transaction>>>()?;

        let versioned_hash_to_blob = create_versioned_hash_blob_mapping(&blobs)?;
        let mut blob_entities: Vec<Blob> = vec![];

        for (tx_hash, versioned_hashes) in tx_hash_to_versioned_hashes.iter() {
            for (i, versioned_hash) in versioned_hashes.iter().enumerate() {
                let blob = *versioned_hash_to_blob.get(versioned_hash).with_context(|| format!("Sidecar not found for blob {i} with versioned hash {versioned_hash} from tx {tx_hash}"))?;

                blob_entities.push(Blob::from((blob, versioned_hash, i, tx_hash)));
            }
        }

        let tx_hashes = transactions_entities
            .iter()
            .map(|tx| tx.hash.to_string())
            .collect::<Vec<String>>();
        let blob_versioned_hashes = blob_entities
            .iter()
            .map(|blob| blob.versioned_hash.to_string())
            .collect::<Vec<String>>();

        blobscan_client
            .index(block_entity, transactions_entities, blob_entities)
            .await
            .map_err(SlotProcessorError::ClientError)?;

        info!(
            target = "slot_processor",
            slot,
            block = execution_block_hash.to_string(),
            transactions = format!("{:?}", tx_hashes),
            blobs = format!("{:?}", blob_versioned_hashes),
            "Block, transactions and blobs indexed successfully!"
        );

        Ok(())
    }
}
