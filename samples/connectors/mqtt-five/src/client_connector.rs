// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Describes a trait that should be implemented for a messaging broker to allow connections from
//! publishers and subscribers.

use std::sync::mpsc::Receiver;

use async_trait::async_trait;

/// Trait implementation needed for communicating with a messaging broker. Utilized by both
/// publishers and subscribers to handle outgoing and incomming messages.
#[async_trait]
pub trait PubSubConnectorClient {
    /// Creates a new instance of the client.
    ///
    /// # Arguments
    ///
    /// * `client_id` - Id to be used to create the broker client.
    /// * `uri` - The uri of the broker that the client is connecting to.
    fn new(client_id: String, uri: String) -> Self;

    /// Function that initiates the connection with the messaging broker.
    async fn connect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Function that ends the connection with the messaging broker.
    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Function that handles publishing data to a topic on the messaging broker.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to publish data to.
    /// * `payload` - The data to publish.
    async fn publish(
        &self,
        topic: String,
        payload: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Function that subscribes to a topic. Returns a stream handle.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to subscribe to.
    async fn subscribe(
        &self,
        topic: String,
    ) -> Result<Receiver<PubSubMessage>, Box<dyn std::error::Error + Send + Sync>>;

    /// Function that unsubscribes from a topic.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to unsubscribe from.
    async fn unsubscribe(
        &self,
        topic: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// A message structure containing the topic and payload information.
pub struct PubSubMessage {
    pub topic: String,
    pub payload: String,
}
