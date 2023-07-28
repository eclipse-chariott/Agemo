// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Collection of methods and objects to help with execution as a publisher.

use log::info;
use sample_mqtt_connector::{
    client_connector::PubSubConnectorClient, mqtt_five_client_connector::MqttFiveClientConnector,
};
use std::{sync::mpsc, time::Duration};
use tokio::task::JoinHandle;

use proto::publisher::v1::SubscriptionInfoResponse;

/// Trait that defines a set of methods that a publisher should implement to enable dynamic topic
/// management.
pub trait DynamicPublisher {
    /// Creates a new dynamic publisher.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Endpoint of the Publisher Server. (ex. "0.0.0.0:50061")
    /// * `pub_sub_url` - Url of the Pub Sub Service. (ex. "http://0.0.0.0:50051")
    /// * `protocol` - Protocol of the Publisher Server. (ex. "grpc+proto")
    fn new(endpoint: String, pub_sub_url: String, protocol: String) -> Self;

    /// Method executed when the topic management callback gets a `START` action from the Pub Sub
    /// Service.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic that has an updated state.
    /// * `generated_topic` - The generated topic associated with the topic above.
    fn on_start_action(&self, topic: String, generated_topic: String);

    /// Method executed when the topic management callback gets a `STOP` action from the Pub Sub
    /// Service.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic that has an updated state.
    /// * `generated_topic` - The generated topic associated with the topic above.
    fn on_stop_action(&self, topic: String, generated_topic: String);

    /// Method executed when the topic management callback gets a `DELETE` action from the Pub Sub
    /// Service.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic that has an updated state.
    /// * `generated_topic` - The generated topic associated with the topic above.
    fn on_delete_action(&self, topic: String, generated_topic: String);
}

/// Spawns a task that publishes simulated data until the Receiver is dropped.
///
/// # Arguments
///
/// * `generated_topic` - The generated topic that will be published to.
/// * `known_topic` - The topic that is associated with the data that is requested to be published.
/// * `recv` - The Receiver for the mpcs stream used to stop publishing to a topic.
/// * `pub_id` - The client id of the publisher that is starting to publish.
/// * `client_info` - The info used to connect and publish to the messaging broker.
/// * `data_fn` - The function gathering the data to publish.
pub fn handle_publish_loop<F>(
    generated_topic: String,
    known_topic: String,
    recv: mpsc::Receiver<String>,
    pub_id: String,
    client_info: SubscriptionInfoResponse,
    data_fn: F,
) -> JoinHandle<()>
where
    F: Fn() -> i64 + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let client: MqttFiveClientConnector =
            PubSubConnectorClient::new(pub_id.clone(), client_info.subscription_endpoint.clone());
        let _response = client.connect();

        // Create messages and publish them.
        info!("Publishing on the topic '({known_topic}) {generated_topic}'.");

        loop {
            let data = data_fn();
            let message = format!("{data}");

            let _res = client
                .publish(generated_topic.clone(), message.to_string())
                .await;

            tokio::time::sleep(Duration::from_secs(1)).await;

            // Only break out of the loop once the connection has been closed.
            match recv.try_recv() {
                Ok(val) => info!("{val:?}"),
                Err(mpsc::TryRecvError::Empty) => continue,
                Err(mpsc::TryRecvError::Disconnected) => break,
            };
        }

        // Disconnect from the broker.
        let _res = client.disconnect().await;

        info!("Stopping publishing on topic '({known_topic}) {generated_topic}'.");
    })
}
