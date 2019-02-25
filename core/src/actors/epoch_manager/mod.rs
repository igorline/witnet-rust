use actix::{Actor, AsyncContext, Context, Recipient, SystemService};

use ansi_term::Color::Purple;

use log::{debug, error, info, warn};

use std::{collections::BTreeMap, time::Duration};

use witnet_config::config::Config;
use witnet_data_structures::chain::Epoch;
use witnet_util::timestamp::{get_timestamp, get_timestamp_nanos};

use crate::actors::messages::{EpochNotification, EpochResult};

mod actor;
mod handlers;

/// Possible errors when getting the current epoch
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EpochManagerError {
    /// Epoch zero time is unknown
    UnknownEpochZero,
    /// Checkpoint period is unknown
    UnknownCheckpointPeriod,
    // Current time is unknown
    // (unused because get_timestamp() cannot fail)
    //UnknownTimestamp,
    /// Checkpoint zero is in the future
    CheckpointZeroInTheFuture(i64),
    /// Overflow when calculating the epoch timestamp
    Overflow,
}

////////////////////////////////////////////////////////////////////////////////////////
// ACTOR BASIC STRUCTURE
////////////////////////////////////////////////////////////////////////////////////////
/// EpochManager actor
#[derive(Default)]
pub struct EpochManager {
    /// Timestamp of checkpoint #0 (the second in which epoch #0 started)
    checkpoint_zero_timestamp: Option<i64>,

    /// Period between checkpoints, in seconds
    checkpoints_period: Option<u16>,

    /// Subscriptions to a particular epoch
    subscriptions_epoch: BTreeMap<Epoch, Vec<Box<dyn SendableNotification>>>,

    /// Subscriptions to all epochs
    subscriptions_all: Vec<Box<dyn SendableNotification>>,

    /// Last epoch that was checked by the epoch monitor process
    last_checked_epoch: Option<Epoch>,
}

/// Required trait for being able to retrieve EpochManager address from system registry
impl actix::Supervised for EpochManager {}

/// Required trait for being able to retrieve EpochManager address from system registry
impl SystemService for EpochManager {}

/// Auxiliary methods for EpochManager actor
impl EpochManager {
    /// Set the timestamp for the start of the epoch zero
    pub fn set_checkpoint_zero(&mut self, timestamp: i64) {
        self.checkpoint_zero_timestamp = Some(timestamp);
    }
    /// Set the checkpoint period (epoch duration)
    pub fn set_period(&mut self, mut period: u16) {
        if period == 0 {
            warn!("Setting the checkpoint period to the minimum value of 1 second");
            period = 1;
        }
        self.checkpoints_period = Some(period);
    }
    /// Calculate the last checkpoint (current epoch) at the supplied timestamp
    pub fn epoch_at(&self, timestamp: i64) -> EpochResult<Epoch> {
        match (self.checkpoint_zero_timestamp, self.checkpoints_period) {
            (Some(zero), Some(period)) => {
                let elapsed = timestamp - zero;
                if elapsed < 0 {
                    Err(EpochManagerError::CheckpointZeroInTheFuture(zero))
                } else {
                    let epoch = elapsed as Epoch / Epoch::from(period);
                    Ok(epoch)
                }
            }
            (None, _) => Err(EpochManagerError::UnknownEpochZero),
            (_, None) => Err(EpochManagerError::UnknownCheckpointPeriod),
        }
    }
    /// Calculate the last checkpoint (current epoch)
    pub fn current_epoch(&self) -> EpochResult<Epoch> {
        let now = get_timestamp();
        self.epoch_at(now)
    }
    /// Calculate the timestamp for a checkpoint (the start of an epoch)
    pub fn epoch_timestamp(&self, epoch: Epoch) -> EpochResult<i64> {
        match (self.checkpoint_zero_timestamp, self.checkpoints_period) {
            // Calculate (period * epoch + zero) with overflow checks
            (Some(zero), Some(period)) => Epoch::from(period)
                .checked_mul(epoch)
                .filter(|&x| x <= Epoch::max_value() as Epoch)
                .map(i64::from)
                .and_then(|x| x.checked_add(zero))
                .ok_or(EpochManagerError::Overflow),
            (None, _) => Err(EpochManagerError::UnknownEpochZero),
            (_, None) => Err(EpochManagerError::UnknownCheckpointPeriod),
        }
    }
    /// Method to process the configuration received from the config manager
    fn process_config(&mut self, ctx: &mut <Self as Actor>::Context, config: &Config) {
        self.set_checkpoint_zero(config.consensus_constants.checkpoint_zero_timestamp);
        self.set_period(config.consensus_constants.checkpoints_period);
        info!(
            "Checkpoint zero timestamp: {}, checkpoints period: {}",
            self.checkpoint_zero_timestamp.unwrap(),
            self.checkpoints_period.unwrap()
        );

        // Start checkpoint monitoring process
        self.checkpoint_monitor(ctx);
    }
    /// Method to compute time remaining to next checkpoint
    fn time_to_next_checkpoint(&self) -> EpochResult<Duration> {
        // Get current timestamp and epoch
        let (now_secs, now_nanos) = get_timestamp_nanos();
        let current_epoch = self.epoch_at(now_secs)?;

        // Get timestamp for the start of next checkpoint
        let next_checkpoint = self.epoch_timestamp(
            current_epoch
                .checked_add(1)
                .ok_or(EpochManagerError::Overflow)?,
        )?;

        // Get number of nanoseconds remaining to the next checkpoint
        let secs = next_checkpoint - now_secs;

        // Check if number of seconds to next checkpoint is valid
        // This number should never be negative with current implementation
        if secs < 0 {
            Err(EpochManagerError::Overflow)
        } else {
            Ok(Duration::new(secs as u64, 1_000_000_000 - now_nanos))
        }
    }
    /// Method to monitor checkpoints and execute some actions on each
    fn checkpoint_monitor(&self, ctx: &mut Context<Self>) {
        // Wait until next checkpoint to execute the periodic function
        ctx.run_later(
            self.time_to_next_checkpoint().unwrap_or_else(|_| {
                Duration::from_secs(u64::from(self.checkpoints_period.unwrap()))
            }),
            move |act, ctx| {
                // Get current epoch
                let current_epoch = match act.current_epoch() {
                    Ok(epoch) => epoch,
                    Err(_) => return,
                };

                // Send message to actors which subscribed to all epochs
                for subscription in &mut act.subscriptions_all {
                    subscription.send_notification(current_epoch);
                }

                // Get all the checkpoints that had some subscription but were skipped for some
                // reason (process sent to background, checkpoint monitor process had no
                // resources to execute in time...)
                let epoch_checkpoints: Vec<_> = act
                    .subscriptions_epoch
                    .range(act.last_checked_epoch.unwrap_or(0)..=current_epoch)
                    .map(|(k, _v)| *k)
                    .collect();

                // Send notifications for skipped checkpoints for subscriptions to a particular
                // epoch
                // Notifications for skipped checkpoints are not sent for subscriptions to all
                // epochs
                for checkpoint in epoch_checkpoints {
                    // Get the subscriptions to the skipped checkpoint
                    if let Some(subscriptions) = act.subscriptions_epoch.remove(&checkpoint) {
                        // Send notifications to subscribers for skipped checkpoints
                        for mut subscription in subscriptions {
                            // TODO: should send messages or just drop?
                            // TODO: send notifications also for subscriptions to all epochs?
                            subscription.send_notification(checkpoint);
                        }
                    }
                }

                // Update last checked epoch
                act.last_checked_epoch = Some(current_epoch);

                debug!("Updated epoch in ChainManager state to #{}", current_epoch);
                info!(
                    "{} We are now in epoch #{}",
                    Purple.bold().paint("[Checkpoints]"),
                    Purple.bold().paint(current_epoch.to_string())
                );

                // Reschedule checkpoint monitor process
                act.checkpoint_monitor(ctx);
            },
        );
    }
}

/// Trait that must follow all notifications that will be sent back to subscriber actors
pub trait SendableNotification: Send {
    /// Send notification back to the subscriber
    fn send_notification(&mut self, current_epoch: Epoch);
}

/// Notification for a particular epoch: instantiated by each actor that subscribes to a particular
/// epoch. Stored in the SubscribeEpoch struct and in the EpochManager as SendableNotification
pub struct SingleEpochSubscription<T: Send> {
    /// Actor recipient, required to send a message back to the subscriber actor
    pub recipient: Recipient<EpochNotification<T>>,

    /// Payload to be sent back to the subscriber actor
    pub payload: Option<T>,
}

/// Implementation of the SendableNotification trait for the SingleEpochSubscription
impl<T: Send> SendableNotification for SingleEpochSubscription<T> {
    /// Function to send notification back to the subscriber
    fn send_notification(&mut self, epoch: Epoch) {
        // Get the payload from the notification
        if let Some(payload) = self.payload.take() {
            // Build an EpochNotification message to send back to the subscriber
            let msg = EpochNotification {
                checkpoint: epoch,
                payload,
            };

            // Send EpochNotification message back to the subscriber
            // TODO: ignore failure?
            match self.recipient.do_send(msg) {
                Ok(()) => {}
                Err(_e) => {}
            };
        } else {
            error!(
                "No payload to be sent back to the subscribed actor for epoch {:?}",
                epoch
            );
        }
    }
}

/// Notification for all epochs: instantiated by each actor that subscribes to all epochs. Stored in
/// the SubscribeAll struct and in the EpochManager as SendableNotification. Requires T to be
/// cloned as this notification is to be sent many times
pub struct AllEpochSubscription<T: Clone + Send> {
    /// Actor recipient, required to send a message back to the subscriber actor
    pub recipient: Recipient<EpochNotification<T>>,

    /// Payload to be sent back to the subscriber actor
    pub payload: T,
}

/// Implementation of the SendableNotification trait for the AllEpochSubscription
impl<T: Clone + Send> SendableNotification for AllEpochSubscription<T> {
    /// Function to send notification back to the subscriber
    fn send_notification(&mut self, epoch: Epoch) {
        // Clone the payload to be sent to the subscriber
        let payload = self.payload.clone();

        // Build an EpochNotification message to send back to the subscriber
        let msg = EpochNotification {
            checkpoint: epoch,
            payload,
        };

        // Send EpochNotification message back to the subscriber
        // TODO: ignore failure?
        match self.recipient.do_send(msg) {
            Ok(()) => {}
            Err(_e) => {}
        };
    }
}
