// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Chariott-enabled subscriber example showing how to get information about and subscribe to a
//! topic following the Pub Sub Service model. Calls Chariott's service discovery to get publisher
//! endpoint information.

use std::{env, sync::Arc};

use async_std::sync::Mutex;
use env_logger::{Builder, Target};
use log::{info, LevelFilter};
use proto::publisher;

use samples_common::{
    chariott_helper::{self, ChariottClient},
    constants,
    subscriber_helper::{self, BrokerRef, TopicRef, EMPTY_TOPIC, SHUTDOWN},
};
use tonic::Status;
use uuid::Uuid;

/// Gets the publisher endpoint from Chariott.
///
/// # Arguments
///
/// * `chariott_url` - The Chariott url.
/// * `namespace` - The namespace to get publisher information about.
/// * `retry_interval_secs` - The interval to wait before retrying the connection.
async fn get_publisher_url_with_retry(
    chariott_client: &mut ChariottClient,
    namespace: &str,
    retry_interval_secs: u64,
) -> Result<String, Status> {
    let service = chariott_helper::get_service_metadata_with_retry(
        chariott_client,
        namespace,
        retry_interval_secs,
        publisher::v1::SCHEMA_KIND,
        publisher::v1::SCHEMA_REFERENCE,
    )
    .await?;

    Ok(service.uri)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Setup logging.
    Builder::new()
        .filter(None, LevelFilter::Info)
        .target(Target::Stdout)
        .init();

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

    // Get publisher endpoint from Chariott.
    let namespace = "sdv.chariott.publisher";
    let retry_interval_secs = 5;

    // Attempt to connect to Chariott.
    let mut chariott_client = chariott_helper::connect_to_chariott_with_retry(
        constants::CHARIOTT_ENDPOINT,
        retry_interval_secs,
    )
    .await?;

    // Wait for publisher service to register with Chariott.
    let publisher_url =
        get_publisher_url_with_retry(&mut chariott_client, namespace, retry_interval_secs).await?;

    // Get subscription information.
    let info = subscriber_helper::get_subscription_info(&publisher_url, &subject, "mqtt").await?;
    {
        let mut topic = topic_handle.lock().await;
        topic.topic = info.topic.clone();
    }

    // Set up the topic stream to receive messages on from the broker (MQTT v5 in this case).
    let id = format!("sub_{}", Uuid::new_v4());
    let stream = subscriber_helper::get_subscription_stream(
        id,
        info.endpoint,
        topic_handle.clone(),
        broker_handle,
    )
    .await?;

    // Print out the messages received by the subscription.
    // This loop will not break unless the stream is broken by the client.
    for msg in stream.into_iter() {
        info!("({}) {}: {}", subject, msg.topic, msg.payload);

        // If deletion message is sent over the subscription then end the program.
        if msg.payload == *"TOPIC DELETED" {
            let mut topic = topic_handle.lock().await;
            topic.topic = EMPTY_TOPIC.to_string();
            let _ = shutdown_sender.send(SHUTDOWN.to_string());
        }
    }

    Ok(())
}
