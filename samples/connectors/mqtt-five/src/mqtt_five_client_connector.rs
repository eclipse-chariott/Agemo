// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! A MQTT v5 client that implements the [PubSubConnectorClient] trait.

use std::{
    collections::HashMap,
    io::ErrorKind,
    process,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
};

use async_trait::async_trait;
use log::{error, info};
use paho_mqtt::{self as mqtt, MQTT_VERSION_5};

use crate::client_connector::{PubSubConnectorClient, PubSubMessage};

/// Alias that maps a topic to a sender stream.
type Subscriptions = HashMap<String, Sender<PubSubMessage>>;

/// Implementation of an MQTT v5 client.
pub struct MqttFiveClientConnector {
    /// Underlying client that handles the mqtt connection.
    client: mqtt::AsyncClient,
    /// Handle to shared subscription map.
    subscriptions: Arc<Mutex<Subscriptions>>,
}

#[async_trait]
impl PubSubConnectorClient for MqttFiveClientConnector {
    fn new(client_id: String, uri: String) -> Self {
        let host = uri.clone();

        let create_opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(host)
            .client_id(client_id.clone())
            .finalize();

        let cli = mqtt::AsyncClient::new(create_opts).unwrap_or_else(|e| {
            error!("Error creating the client: {e:?}");
            process::exit(1); // TODO: gracefully handle with retry?
        });

        let subscriptions = Arc::new(Mutex::new(Subscriptions::new()));

        let cb_subscriptions = subscriptions.clone();

        cli.set_message_callback(move |_cli, msg| {
            if let Some(msg) = msg {
                let topic = msg.topic();
                let payload = msg.payload_str();

                let sub_lock = cb_subscriptions.lock().unwrap();

                if let Some(topic_ch) = sub_lock.get(topic) {
                    let message = PubSubMessage {
                        topic: topic.to_string(),
                        payload: payload.to_string(),
                    };

                    // TODO: handle send error.
                    let _res = topic_ch.send(message);
                }
            }
        });

        info!("Created client with id: {client_id} and connection_uri: {uri}");

        MqttFiveClientConnector {
            client: cli,
            subscriptions,
        }
    }

    async fn connect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let id = self.client.client_id();
        let lwt_string = format!("client_id: {} has lost connection", id);
        // TODO: Make this more generic so that it can be used in the case of Subscriber disconnect.
        let lwt = mqtt::Message::new("publisher/disconnect", lwt_string, mqtt::QOS_1);

        let conn_opts = mqtt::ConnectOptionsBuilder::with_mqtt_version(MQTT_VERSION_5)
            .clean_start(false)
            .properties(mqtt::properties![mqtt::PropertyCode::SessionExpiryInterval => 3600])
            .will_message(lwt)
            .finalize();

        if let Err(err) = self.client.connect(conn_opts).wait() {
            error!("Unable to connect: {err}");
            process::exit(1);
        }

        Ok(())
    }

    async fn disconnect(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.client.is_connected() {
            if let Err(err) = self.client.disconnect(None).wait() {
                error!("Error disconnecting: {err}");
                process::exit(1);
            }
        }

        Ok(())
    }

    async fn publish(
        &self,
        topic: String,
        payload: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.client.is_connected() {
            if let Err(err) = self.connect().await {
                error!("Error connecting: {err}");
                process::exit(1);
            }
        }

        let msg = mqtt::Message::new(topic.clone(), payload, mqtt::QOS_1);

        self.client.publish(msg).await?;

        Ok(())
    }

    async fn subscribe(
        &self,
        topic: String,
    ) -> Result<Receiver<PubSubMessage>, Box<dyn std::error::Error + Send + Sync>> {
        // This first validates that the topic requested can be subscribed to.
        // TODO: custom error
        self.client
            .subscribe(&topic, 1)
            .await
            .map_err(|e| Box::new(std::io::Error::new(ErrorKind::Other, e.to_string())))?;

        let mut sub_lock = self.subscriptions.lock().unwrap();
        let (sender, receiver) = mpsc::channel::<PubSubMessage>();

        sub_lock.insert(topic.clone(), sender);

        Ok(receiver)
    }

    async fn unsubscribe(
        &self,
        topic: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client.unsubscribe(topic).await?;

        Ok(())
    }
}
