// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Module containing trait that a publish/subscribe connector must implement.
//!
//! The [`PubSubConnector`] trait is required to be implemented by a broker connector to enable the
//! dynamic topic management in the pub sub service. Below is a list of requirements that a broker
//! must meet to be integrated with no changes to the service:
//! - Broker must provide a way to monitor subscribe requests to a topic.
//! - Broker must provide a way to monitor unsubscribe requests to a topic.
//! - Broker must provide a way to monitor clients that disconnect from the broker unexpectedly.
//!   This is to enable the service to publish a [`TOPIC_DELETED_MSG`] to notify subscribers to
//!   drop the topic.
//!
//! If a broker you want to use does not meet the above requirements, please reach out via an
//! issue on GitHub.

use async_trait::async_trait;
use std::{fmt, sync::mpsc};

/// Constant defining the message sent over a topic channel notifying any subscribers that a topic
/// has been deleted.
pub const TOPIC_DELETED_MSG: &str = "TOPIC DELETED";

/// Enum defining the protocol type used by the messaging broker.
#[derive(Debug, Clone, Copy)]
pub enum PubSubProtocol {
    /// Represents the MQTT protocol.
    Mqtt,
}

/// Enum representing an action that happens in the messaging broker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PubSubAction {
    /// Represents a subscribe to a topic.
    Subscribe,
    /// Represents an unsubscribe to a topic.
    Unsubscribe,
    /// Represents a notification that a topic has no subscribers after a period of time.
    Timeout,
    /// Represents a deletion of a topic.
    Delete,
    /// Represents an unclean publisher disconnect.
    PubDisconnect,
}

impl fmt::Display for PubSubProtocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for PubSubAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let string = format!("{self:?}").to_uppercase();
        write!(f, "{string}")
    }
}

/// Structure defining a message returned from the broker connector when an action happens.
#[derive(Debug)]
pub struct MonitorMessage {
    /// A string that provides the context relevant to the action that triggered the message.
    pub context: String,
    /// The action that triggered the message from the broker connector.
    pub action: PubSubAction,
}

/// Trait that needs to be implmented by a broker connector for the pub sub service to get
/// the necessary information from the messaging broker to implement dynamic topic management.
#[async_trait]
pub trait PubSubConnector {
    /// Creates a new instance of the struct implementing this trait.
    ///
    /// # Arguments
    ///
    /// * `client_id` - Id to be used to create the broker client.
    /// * `endpoint` - The endpoint of the broker that the client is connecting to.
    fn new(client_id: String, endpoint: String) -> Self;

    /// Function that monitors the messaging broker for changes and forwards those changes back
    /// over the callback channel.
    ///
    /// This function monitors changes to topics and connections on the messaging broker. Every
    /// update to the broker is sent to the provided callback channel in the format of a
    /// [`MonitorMessage`]. The types of updates that are monitored are listed out in the
    /// [`PubSubAction`] enum.
    ///
    /// # Arguments
    ///
    /// * `cb_channel` - Callback channel used to forward messages from the connector to the rest
    ///                  of the pub sub service logic.
    async fn monitor_topics(
        &mut self,
        cb_channel: mpsc::Sender<MonitorMessage>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Function that deletes a topic from the messaging broker.
    ///
    /// This function deletes a topic from the messaging broker. In addition, it sends a topic
    /// deletion message across the topic channel to inform any subscribers that the topic is being
    /// deleted.
    ///
    /// # Arguments
    ///
    /// * `topic` - Generated topic to be deleted from the service.
    /// * `deletion_msg` - Deletion message to be sent to any subscribers on the given topic.
    async fn delete_topic(
        &self,
        topic: String,
        deletion_msg: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Function that is used to send a [`MonitorMessage`] to the given channel.
///
/// This function is a common function that can be utilized while implementing the
/// [`PubSubConnector`] trait.
///
/// # Arguments
///
/// * `update_msg` - Message collected by the connector related to an update to a topic.
/// * `channel` - Channel used to forward the given update_msg to the rest of the Pub Sub Service.
pub fn update_topic_information(update_msg: MonitorMessage, channel: mpsc::Sender<MonitorMessage>) {
    channel.send(update_msg).unwrap();
}

#[cfg(test)]
mod pubsub_action_tests {
    use super::*;

    #[test]
    fn action_to_string() {
        assert_eq!("SUBSCRIBE".to_string(), PubSubAction::Subscribe.to_string());
        assert_eq!(
            "UNSUBSCRIBE".to_string(),
            PubSubAction::Unsubscribe.to_string()
        );
        assert_eq!("TIMEOUT".to_string(), PubSubAction::Timeout.to_string());
        assert_eq!("DELETE".to_string(), PubSubAction::Delete.to_string());
        assert_eq!(
            "PUBDISCONNECT".to_string(),
            PubSubAction::PubDisconnect.to_string()
        );
    }
}
