// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Implements the [DynamicPublisher] trait and the server side implementation of the
//! [publisher.proto](proto::publisher) interface.
//!
//! The DynamicPublisher trait defines three methods that execute on the three possible updates
//! from the Pub Sub Service (START, STOP and DELETE)

use log::info;
use proto::publisher::v1::{
    publisher_server::Publisher, ManageTopicRequest, ManageTopicResponse, SubscriptionInfoRequest,
    SubscriptionInfoResponse,
};
use samples_common::{
    data_generator,
    pub_sub_service_helper::{self, TopicAction},
    publisher_helper::{self, DynamicPublisher},
    topic_store::TopicStore,
};
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};
use tonic::{Request, Response, Status};

/// Default endpoint for the simple publisher.
pub const ENDPOINT: &str = "[::1]:50061";

/// Alias for the active topics hashmap, correlating a topic with a sender channel used to publish
/// to the messaging broker.
pub type ActiveTopicsMap = HashMap<String, mpsc::Sender<String>>;

/// Base structure for the publisher gRPC service.
#[derive(Debug, Default)]
pub struct PublisherImpl {
    /// Id of the publisher.
    pub id: String,
    /// Handle pointing to a shared active topics map.
    pub active_topics_map: Arc<Mutex<ActiveTopicsMap>>,
    /// Store that maps the dynamically created topic to a topic known to the publisher.
    pub topics_store: TopicStore,
    /// The url of the Pub Sub Service.
    pub pub_sub_url: String,
}

impl PublisherImpl {
    /// Creates a new instance of the PublisherImpl struct.
    ///
    /// # Arguments
    ///
    /// * `pub_sub_url` - Url of the Pub Sub Service. (ex. "http://\[::1\]:50051")
    pub fn new(pub_sub_url: String) -> Self {
        PublisherImpl {
            id: format!("pub_{}", uuid::Uuid::new_v4()),
            topics_store: TopicStore::new(),
            active_topics_map: Arc::new(Mutex::new(ActiveTopicsMap::new())),
            pub_sub_url,
        }
    }
}

impl DynamicPublisher for PublisherImpl {
    /// Creates a new instance of the DynamicPublisher by calling the `PublisherImpl::new` method.
    ///
    /// # Arguments
    ///
    /// * `pub_sub_url` - Url of the Pub Sub Service. (ex. "http://\[::1\]:50051")
    fn new(pub_sub_url: String) -> Self {
        PublisherImpl::new(pub_sub_url)
    }

    /// Action taken by the publisher when a START action is received from the Pub Sub Service.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic known to the publisher that is associated with the generated topic.
    /// * `generated_topic` - The generated topic from the Pub Sub Service.
    fn on_start_action(&self, topic: String, generated_topic: String) {
        // Initialize a client with a disconnect channel
        let (send, recv) = mpsc::channel::<String>();

        // Add disconnect channel to active topics map with the key being the topic known to the publisher.
        {
            self.active_topics_map
                .lock()
                .unwrap()
                .insert(topic.clone(), send);
        }

        // Update topic metadata.
        let topic_metadata = self
            .topics_store
            .update_topic_action(&topic, TopicAction::Start);

        let client_info = topic_metadata.unwrap().subscription_info;

        // Start publishing in a separate thread. Uses a simple data generator from the common folder.
        let _handle = publisher_helper::handle_publish_loop(
            generated_topic,
            topic,
            recv,
            self.id.clone(),
            client_info,
            data_generator::get_data,
        );
    }

    /// Action taken by the publisher when a STOP action is received from the Pub Sub Service.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic known to the publisher that is associated with the generated topic.
    /// * `generated_topic` - The generated topic from the Pub Sub Service.
    fn on_stop_action(&self, topic: String, generated_topic: String) {
        // Remove topic from active list and call disconnect channel for topic thread.
        // Then set the last active timestamp of the action actually removed the topic.
        {
            if let Some(sender) = self.active_topics_map.lock().unwrap().remove(&topic) {
                drop(sender);
                self.topics_store.set_topic_last_active(&topic);
            }
        }

        // The service will keep sending 'STOP' pings as long as the topic exists and there are no subscribers.
        // This is an example of how a publisher could choose to delete the topic on a stop message.
        if let Some(topic_metadata) = self.topics_store.get_topic_metadata(&topic) {
            let threshold = Duration::from_secs(20);

            if topic_metadata.last_active.elapsed().as_secs() > threshold.as_secs() {
                // Remove topic from store.
                self.topics_store.remove_topic(&topic, &generated_topic);

                info!("Deleting topic '({topic}) {generated_topic}'.");

                // Call delete topic from the Pub Sub Service.
                let url = self.pub_sub_url.clone();
                let _handle = tokio::spawn(async move {
                    pub_sub_service_helper::delete_topic(url, generated_topic.clone()).await
                });
            }
        }
    }

    /// Action taken by the publisher when a DELETE action is received from the Pub Sub Service.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic known to the publisher that is associated with the generated topic.
    /// * `generated_topic` - The generated topic from the Pub Sub Service.
    fn on_delete_action(&self, topic: String, generated_topic: String) {
        // If the Pub Sub Service initiates a DELETE, then a long-lived timeout has been reached and
        // no one has touched the topic in a long time. In this case, it has been dropped from the service,
        // so just remove the topic from the active lists.

        // Remove topic from lists.
        self.topics_store.remove_topic(&topic, &generated_topic);
        if let Some(sender) = self.active_topics_map.lock().unwrap().remove(&topic) {
            drop(sender);
        }
    }
}

#[tonic::async_trait]
impl Publisher for PublisherImpl {
    /// Provides subscription information based on the given request.
    ///
    /// This function returns the necessary info needed to subscribe to a data stream based on the
    /// request.
    ///
    /// # Arguments
    /// * `request` - Contains the requested subject to get subscription information about.
    async fn get_subscription_info(
        &self,
        request: Request<SubscriptionInfoRequest>,
    ) -> Result<Response<SubscriptionInfoResponse>, Status> {
        // Extract the topic from the request. For simplicity in this example the subject
        // correlates directly with a topic.
        let requested_topic = request.into_inner().subject;
        info!("Got request for subscription info on subject '{requested_topic}'.");

        // If there is already a dynamic topic created for the subject then shortcut and return
        // that subscription info.
        if let Some(topic_metadata) = self.topics_store.get_topic_metadata(&requested_topic) {
            return Ok(Response::new(topic_metadata.subscription_info));
        }

        // Otherwise, call Pub Sub service and get the topic and subscription information.
        let topic_subscription_info = pub_sub_service_helper::create_topic(
            self.pub_sub_url.clone(),
            self.id.clone(),
            String::from(ENDPOINT),
            String::from("grpc"),
        )
        .await?;

        // Add new topic information to the topic maps.
        self.topics_store
            .add_topic(requested_topic, topic_subscription_info.clone());

        // Return response with how to subscribe to publisher.
        let response = Response::new(topic_subscription_info);

        Ok(response)
    }

    /// Allows for topic management by the Pub Sub Service.
    ///
    /// Callback implemented by the publisher and utilized by the Pub Sub Service to provide
    /// updates about a dynamically created topic to the publisher. The actions taken by the
    /// publisher are implemented by the [`DynamicPublisher`] trait.
    ///
    /// # Arguments
    ///
    /// * `request` - Contains a topic and relevant update information.
    async fn manage_topic_callback(
        &self,
        request: Request<ManageTopicRequest>,
    ) -> Result<Response<ManageTopicResponse>, Status> {
        // Unwrap the request.
        let manage_req = request.into_inner();
        let action = TopicAction::from_str(manage_req.action.as_str())
            .map_err(|e| Status::not_found(format!("no valid action was found: {e}")))?;
        let generated_topic = manage_req.topic;

        // Get known topic based on the passed in generated topic.
        let topic = self
            .topics_store
            .get_generated_topic_mapping(&generated_topic)?;

        info!("Executing action '{action}' for topic '({topic}) {generated_topic}'.");

        // Execute action for a topic based on the type.
        match action {
            TopicAction::Start => self.on_start_action(topic, generated_topic),
            TopicAction::Stop => self.on_stop_action(topic, generated_topic),
            TopicAction::Delete => self.on_delete_action(topic, generated_topic),
            _ => return Err(Status::already_exists(generated_topic)),
        }

        info!("Successfully executed action.");

        Ok(Response::new(ManageTopicResponse {}))
    }
}
