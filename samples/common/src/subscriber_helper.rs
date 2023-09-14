// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Collection of methods and objects to help with execution as a subscriber.

use std::{
    process,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
};

use async_std::sync::Mutex;
use log::{error, info};
use samples_proto::sample_publisher::v1::{sample_publisher_client::SamplePublisherClient, SubscriptionInfoRequest};
use sample_mqtt_connector::{
    client_connector::{PubSubConnectorClient, PubSubMessage},
    mqtt_five_client_connector::MqttFiveClientConnector,
};
use serde_json::Value;

/// Shutdown constant used to tell the service to shut down over an mpsc channel.
pub const SHUTDOWN: &str = "shutdown";
/// Empty topic constant used to initialize the [`TopicRef`].
pub const EMPTY_TOPIC: &str = "";

/// A reference used to share the connector instance between the shutdown task and main thread.
pub struct BrokerRef {
    /// The messaging broker client.
    pub client: Option<MqttFiveClientConnector>,
}

/// A reference used to share the topic name between the shutdown task and main thread.
pub struct TopicRef {
    /// The topic name, initializes as [`EMPTY_TOPIC`].
    pub topic: String,
}

/// Object connecting the subscription uri and a topic.
pub struct SubscriptionInfo {
    /// The uri to subscribe to (generally the messaging broker uri).
    pub uri: String,
    /// The topic to subscribe to.
    pub topic: String,
}

/// Gets the subscription information from the publisher client.
///
/// # Arguments
///
/// * `pub_uri` - The uri of the publisher of the data.
/// * `subject` - The subject to request data about.
/// * `expected_protocol` - The protocol expected for the subscription.
pub async fn get_subscription_info(
    pub_uri: &str,
    subject: &str,
    expected_protocol: &str,
) -> Result<SubscriptionInfo, Box<dyn std::error::Error + Send + Sync>> {
    info!("Requesting subject: {}", subject);

    let pub_client_result = SamplePublisherClient::connect(pub_uri.to_string()).await;

    // TODO: Handle error
    let mut pub_client = pub_client_result.unwrap();

    // Get subscription info from publisher.
    let sub_request = SubscriptionInfoRequest {
        subject: subject.to_string(),
    };
    let sub_response = pub_client.get_subscription_info(sub_request).await?;
    let sub_info = sub_response.into_inner();
    let protocol = sub_info.protocol_kind;
    let uri = sub_info.subscription_uri;

    // If protocol returned is something the subscriber can't handle, then exit.
    if protocol != *expected_protocol {
        error!("Unable to communicate with pub sub, expected protocol mqtt, but protocol is {protocol}.");
        process::exit(1);
    }

    // Process subscription metadata to get topic name to subscribe to.
    let metadata_json: Value = serde_json::from_str(&sub_info.subscription_metadata).unwrap();
    let topic = metadata_json["topic"].as_str().unwrap().to_string();

    Ok(SubscriptionInfo { uri, topic })
}

/// Gets the subscription stream from the broker.
///
/// # Arguments
///
/// * `client_id` - The id of the subscriber service.
/// * `uri` - The uri of the messaging broker.
/// * `topic_handle` - The shared reference handle of the topic.
/// * `broker_handle` - The shared reference handle of the broker.
pub async fn get_subscription_stream(
    client_id: String,
    uri: String,
    topic_handle: Arc<Mutex<TopicRef>>,
    broker_handle: Arc<Mutex<BrokerRef>>,
) -> Result<Receiver<PubSubMessage>, Box<dyn std::error::Error + Send + Sync>> {
    let topic = topic_handle.lock().await;
    let mut broker = broker_handle.lock().await;

    broker.client = Some(PubSubConnectorClient::new(client_id, uri));
    broker
        .client
        .as_ref()
        .expect("broker exists.")
        .connect()
        .await?;

    // A stream is returned from the subscribe call, which is used to get the messages from the broker.
    info!("Subscribing...");
    broker
        .client
        .as_ref()
        .expect("broker exists.")
        .subscribe(topic.topic.clone())
        .await
}

/// Gracefully shuts down the sample when Ctrl+C is called.
///
/// # Arguments
///
/// * `broker_handle` - The shared reference handle of the broker.
/// * `topic_handle` - The shared reference handle of the topic.
pub async fn handle_ctrlc_shutdown(
    broker_handle: Arc<Mutex<BrokerRef>>,
    topic_handle: Arc<Mutex<TopicRef>>,
) -> Sender<String> {
    let (shutdown_sender, shutdown_receiver) = mpsc::channel::<String>();

    // Used to shut down the program if a topic is closed by the publisher.
    let topic_deleted_sender = shutdown_sender.clone();

    // This captures the ctrl+c used to end a program and allows for the subscriber to gracefully exit.
    ctrlc::set_handler(move || {
        let _ = shutdown_sender.send(SHUTDOWN.to_string());
    })
    .expect("Error setting Ctrl-C handler");

    // Handles the graceful shutdown of the subscriber. If there is a broker client
    // then it unsubscribes and disconnects the client before closing the program.
    let _shutdown_handle = tokio::spawn(async move {
        let shutdown_recv = shutdown_receiver
            .recv()
            .unwrap_or_else(|_| SHUTDOWN.to_string());

        info!("{shutdown_recv} request received.");
        let topic = topic_handle.lock().await;
        let broker = broker_handle.lock().await;
        if broker.client.is_some() {
            if topic.topic.ne(EMPTY_TOPIC) {
                info!("Unsubscribing...");
                let _ = broker
                    .client
                    .as_ref()
                    .expect("broker exists")
                    .unsubscribe(topic.topic.clone())
                    .await;
            }

            info!("Disconnecting...");
            let _ = broker
                .client
                .as_ref()
                .expect("broker exists")
                .disconnect()
                .await;
        }

        process::exit(0);
    });

    topic_deleted_sender
}
