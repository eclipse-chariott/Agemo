// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Implements the [DynamicPublisher] trait and the server side implementation of the
//! [publisher.proto](proto::publisher) interface.
//!
//! The DynamicPublisher trait defines three methods that execute on the three possible updates
//! from the Pub Sub Service (START, STOP and DELETE).

use log::info;
use samples_common::{
    data_generator,
    pub_sub_service_helper::{self, TopicAction},
    publisher_helper::{self, DynamicPublisher},
    topic_store::{TopicMetadata, TopicStore},
};
use samples_proto::{
    publisher::v1::{
        publisher_callback_server::PublisherCallback, ManageTopicRequest, ManageTopicResponse,
    },
    sample_publisher::v1::{
        sample_publisher_server::SamplePublisher, SubscriptionInfoRequest, SubscriptionInfoResponse,
    },
};
use std::{
    str::FromStr,
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};
use tonic::{Request, Response, Status};

/// Base structure for the publisher gRPC service.
#[derive(Clone, Debug, Default)]
pub struct PublisherImpl {
    /// Id of the publisher.
    pub id: String,
    /// The authority of the publisher.
    pub authority: String,
    /// The protocol used to communicate with the publisher.
    pub protocol: String,
    /// Store that maps the dynamically created topic to a topic known to the publisher.
    pub topic_store: Arc<Mutex<TopicStore>>,
    /// The uri of the Pub Sub Service.
    pub pub_sub_uri: String,
}

impl PublisherImpl {
    /// Creates a new instance of the PublisherImpl struct.
    ///
    /// # Arguments
    ///
    /// * `authority` - Authority of the Publisher Server. (ex. "0.0.0.0:50061")
    /// * `pub_sub_uri` - URI of the Pub Sub Service. (ex. "http://0.0.0.0:50051")
    /// * `protocol` - Protocol of the Publisher Server. (ex. "grpc+proto")
    pub fn new(authority: String, pub_sub_uri: String, protocol: String) -> Self {
        PublisherImpl {
            id: format!("pub_{}", uuid::Uuid::new_v4()),
            authority,
            protocol,
            topic_store: Arc::new(Mutex::new(TopicStore::new())),
            pub_sub_uri,
        }
    }
}

impl DynamicPublisher for PublisherImpl {
    /// Creates a new instance of the DynamicPublisher by calling the `PublisherImpl::new` method.
    ///
    /// # Arguments
    ///
    /// * `authority` - Authority of the Publisher Server. (ex. "0.0.0.0:50061")
    /// * `pub_sub_uri` - URI of the Pub Sub Service. (ex. "http://0.0.0.0:50051")
    /// * `protocol` - Protocol of the Publisher Server. (ex. "grpc+proto")
    fn new(authority: String, pub_sub_uri: String, protocol: String) -> Self {
        PublisherImpl::new(authority, pub_sub_uri, protocol)
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

        let topic_metadata: Option<TopicMetadata>;

        // Activate topic in store.
        {
            topic_metadata = self
                .topic_store
                .lock()
                .unwrap()
                .activate_topic(&topic, send);
        }

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
        let topic_store = self.topic_store.lock().unwrap();

        topic_store.deactivate_topic(&topic);

        // The service will keep sending 'STOP' pings as long as the topic exists and there are no subscribers.
        // This is an example of how a publisher could choose to delete the topic on a stop message.
        if let Some(topic_metadata) = topic_store.get_topic_metadata(&topic) {
            let threshold = Duration::from_secs(20);

            if topic_metadata.last_active.elapsed().as_secs() > threshold.as_secs() {
                // Remove topic from store.
                topic_store.remove_topic(&topic, &generated_topic);

                info!("Deleting topic '({topic}) {generated_topic}'.");

                // Call delete topic from the Pub Sub Service.
                let uri = self.pub_sub_uri.clone();
                let _handle = tokio::spawn(async move {
                    pub_sub_service_helper::delete_topic(uri, generated_topic.clone()).await
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
        let topic_store = self.topic_store.lock().unwrap();

        topic_store.deactivate_topic(&topic);
        topic_store.remove_topic(&topic, &generated_topic);
    }
}

#[tonic::async_trait]
impl PublisherCallback for PublisherImpl {
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
        let topic: String;
        {
            topic = self
                .topic_store
                .lock()
                .unwrap()
                .get_generated_topic_mapping(&generated_topic)?;
        }

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

#[tonic::async_trait]
impl SamplePublisher for PublisherImpl {
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
        // Extract the requested topic from the request. For simplicity in this example the subject
        // correlates directly with a topic.
        let requested_topic = request.into_inner().subject;
        info!("Got request for subscription info on subject '{requested_topic}'.");

        // If there is already a dynamic topic created for the subject then shortcut and return
        // that subscription info.
        {
            if let Some(topic_metadata) = self
                .topic_store
                .lock()
                .unwrap()
                .get_topic_metadata(&requested_topic)
            {
                return Ok(Response::new(topic_metadata.subscription_info));
            }
        }

        // Otherwise, call Pub Sub Service and get the topic and subscription information.
        let topic_subscription_info = pub_sub_service_helper::create_topic(
            self.pub_sub_uri.clone(),
            self.id.clone(),
            self.authority.clone(),
            String::from("grpc"),
        )
        .await?;

        // Add new topic information to the topic maps.
        {
            self.topic_store
                .lock()
                .unwrap()
                .add_topic(requested_topic, topic_subscription_info.clone());
        }

        // Return response with how to subscribe to publisher.
        let response = Response::new(topic_subscription_info);

        Ok(response)
    }
}
