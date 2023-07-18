// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Chariott-enabled subscriber example showing how to get information about and subscribe to a
//! topic following the Pub Sub Service model. Calls Chariott's service discovery to get publisher
//! endpoint information.

use std::{env, sync::Arc, thread, time::Duration};

use async_std::sync::Mutex;
use env_logger::{Builder, Target};
use log::{info, warn, LevelFilter};
use proto::publisher;
use sample_chariott_connector::chariott_client::ChariottClient;
use samples_common::{
    constants,
    subscriber_helper::{self, BrokerRef, TopicRef, EMPTY_TOPIC, SHUTDOWN},
};
use tonic::{Code, Status};
use uuid::Uuid;

/// Gets the publisher endpoint from Chariott.
///
/// # Arguments
///
/// * `chariott_url` - The Chariott url.
/// * `namespace` - The namespace to get publisher information about.
async fn get_publisher_url(chariott_url: &str, namespace: &str) -> Result<Option<String>, Status> {
    // Check if publisher exists.
    let mut chariott_client = ChariottClient::new(chariott_url.to_string()).await?;

    let result = chariott_client
        .discover(namespace)
        .await
        .or_else(|status| {
            if status.code() == Code::NotFound || status.code() == Code::Unavailable {
                Ok(None)
            } else {
                Err(status)
            }
        })?
        .and_then(|services| {
            for service in services {
                if service.schema_kind == publisher::v1::SCHEMA_KIND
                    && service.schema_reference == publisher::v1::SCHEMA_REFERENCE
                {
                    return Some(service.url);
                }
            }
            None
        });

    Ok(result)
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

    // Wait for publisher service to register with Chariott.
    let mut publisher_url = None;

    while publisher_url.is_none() {
        let mut reason = String::new();

        publisher_url = get_publisher_url(constants::CHARIOTT_ENDPOINT, namespace)
            .await
            .transpose()
            .or_else(|| {
                reason = format!("No publisher service found at '{namespace}'");
                None
            })
            .and_then(|res| match res {
                Ok(val) => Some(val),
                Err(e) => {
                    if e.code() == Code::Unavailable {
                        reason = String::from("No chariott service found");
                    } else {
                        reason = format!("Chariott request failed with '{e:?}'");
                    }
                    None
                }
            })
            .or_else(|| {
                let secs = 5;
                warn!("{reason}, retrying in {secs} seconds...");
                thread::sleep(Duration::from_secs(secs));
                None
            });
    }

    // Get subscription information.
    let info =
        subscriber_helper::get_subscription_info(&publisher_url.unwrap(), &subject, "mqtt").await?;
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
