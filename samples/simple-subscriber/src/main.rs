// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Simple subscriber example showing how to get information about and subscribe to a topic
//! following the Pub Sub Service model.

use std::{env, sync::Arc};

use async_std::sync::Mutex;
use env_logger::{Builder, Target};
use log::{info, LevelFilter};
use samples_common::{
    load_config::{
        load_settings, CommunicationConstants, SimpleSubscriberServiceSettings, CONFIG_FILE,
        CONSTANTS_FILE,
    },
    subscriber_helper::{self, BrokerRef, TopicRef, EMPTY_TOPIC, SHUTDOWN},
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Setup logging.
    Builder::new()
        .filter(None, LevelFilter::Info)
        .target(Target::Stdout)
        .init();

    // Load in settings for service.
    let settings = load_settings::<SimpleSubscriberServiceSettings>(CONFIG_FILE);
    let communication_consts = load_settings::<CommunicationConstants>(CONSTANTS_FILE);

    // Instantiate shared broker and shared topic references.
    let broker_handle: Arc<Mutex<BrokerRef>> = Arc::new(Mutex::new(BrokerRef { client: None }));
    let topic_handle: Arc<Mutex<TopicRef>> = Arc::new(Mutex::new(TopicRef {
        topic: EMPTY_TOPIC.to_string(),
    }));

    // Setup shutdown watcher to smoothly shutdown service.
    let shutdown_sender =
        subscriber_helper::handle_ctrlc_shutdown(broker_handle.clone(), topic_handle.clone()).await;

    // Subject to get data on.
    let default_subject = "test_topic".to_string();
    let subject = env::args().nth(1).unwrap_or(default_subject);

    // Convert the publisher authority from the configuration settings to a uri.
    let publisher_authority = settings.publisher_authority;
    let publisher_uri = format!("http://{publisher_authority}");

    // Get subscription information from the publisher for the requested subject.
    let info = subscriber_helper::get_subscription_info(
        &publisher_uri,
        &subject,
        &communication_consts.mqtt_v5_kind,
    )
    .await?;
    {
        let mut topic = topic_handle.lock().await;
        topic.topic = info.topic.clone();
    }

    // Set up the topic stream to receive messages on from the broker (MQTT v5 in this case).
    let id = format!("sub_{}", Uuid::new_v4());
    let stream = subscriber_helper::get_subscription_stream(
        id,
        info.uri,
        topic_handle.clone(),
        broker_handle,
    )
    .await?;

    // Print out the messages received by the subscription.
    // This loop will not break unless the stream is broken by the client.
    for msg in stream.into_iter() {
        // Record the message received on the stream.
        info!("({subject}) {}: {}", msg.topic, msg.payload);

        // If deletion message is sent over the subscription then end the program.
        if msg.payload == communication_consts.topic_deletion_message {
            let mut topic = topic_handle.lock().await;
            topic.topic = EMPTY_TOPIC.to_string();
            let _ = shutdown_sender.send(SHUTDOWN.to_string());
        }
    }

    Ok(())
}
