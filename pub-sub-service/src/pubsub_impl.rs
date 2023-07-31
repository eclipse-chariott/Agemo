// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Module containing gRPC service implementation based on [`proto::pubsub`].
//!
//! Provides a gRPC endpoint for external services to interact with to create and manage
//! dynamically created topics.

use log::info;
use std::sync::{Arc, Mutex};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use proto::pubsub::v1::pub_sub_server::PubSub;
use proto::pubsub::v1::{
    CreateTopicRequest, CreateTopicResponse, DeleteTopicRequest, DeleteTopicResponse,
};

use crate::topic_manager::{ActiveTopicsMap, TopicMetadata};

/// Base structure for the pub sub gRPC service.
pub struct PubSubImpl {
    /// Handle that points to a shared active topics map.
    pub active_topics: Arc<Mutex<ActiveTopicsMap>>,
    /// The uri of the messaging broker.
    pub uri: String,
    /// The messaging protocol used by the messaging broker.
    pub protocol: String,
}

#[tonic::async_trait]
impl PubSub for PubSubImpl {
    /// Creates a dynamic topic based on the given request for a publisher.
    ///
    /// This function creates a dynamic topic based on a [`CreateTopicRequest`]. Returns a
    /// [`CreateTopicResponse`].
    ///
    /// # Arguments
    ///
    /// * `request` - The information needed to create a new topic.
    async fn create_topic(
        &self,
        request: Request<CreateTopicRequest>,
    ) -> Result<Response<CreateTopicResponse>, Status> {
        let request_inner = request.into_inner();
        let cb = request_inner.management_callback.clone();
        let pub_id = request_inner.publisher_id;
        info!("Got a request to create topic from '{pub_id}'.");

        let gen_topic = Uuid::new_v4().to_string();

        // Create new topic and add to active topics list. This will start tracking
        // the generated topic until the requestor decides to delete the topic.
        {
            let metadata = TopicMetadata::new(pub_id, 0, Some(cb));
            self.active_topics
                .lock()
                .unwrap()
                .insert(gen_topic.clone(), metadata);
        }

        let reply = CreateTopicResponse {
            generated_topic: gen_topic,
            broker_uri: self.uri.clone(),
            broker_protocol: self.protocol.clone(),
        };

        Ok(Response::new(reply))
    }

    /// Deletes the given topic for a publisher.
    ///
    /// Deletes a topic for a publisher by marking the requested topic for deletion in the shared
    /// active topics list. This is then handled by the logic in the
    /// [`TopicManager`][crate::topic_manager::TopicManager].
    ///
    /// # Arguments
    ///
    /// * `request` - The information needed to delete a topic.
    async fn delete_topic(
        &self,
        request: Request<DeleteTopicRequest>,
    ) -> Result<Response<DeleteTopicResponse>, Status> {
        let request_inner = request.into_inner();
        let topic = request_inner.topic;
        info!("Got a request to delete topic '{topic}.'");

        let mut curr_topics = self.active_topics.lock().unwrap();

        if let Some(t) = curr_topics.get_mut(&topic) {
            t.delete(); // Marks topic for deletion.
        }

        Ok(Response::new(DeleteTopicResponse {}))
    }
}

#[cfg(test)]
mod pubsub_impl_tests {
    use super::*;

    #[tokio::test]
    async fn generate_topic_test() {
        let expected_cb = "test_cb".to_string();
        let expected_management_protocol = "test_mgmt_protocol".to_string();
        let expected_pub_id = "pub_test".to_string();
        let expected_uri = "test_broker".to_string();
        let expected_protocol = "test_protocol".to_string();
        let expected_metadata =
            TopicMetadata::new(expected_pub_id.clone(), 0, Some(expected_cb.clone()));

        let test_topic_map = Arc::new(Mutex::new(ActiveTopicsMap::new()));

        let pubsub = PubSubImpl {
            active_topics: test_topic_map.clone(),
            uri: expected_uri.clone(),
            protocol: expected_protocol.clone(),
        };

        let request = Request::new(CreateTopicRequest {
            publisher_id: expected_pub_id.clone(),
            management_callback: expected_cb.clone(),
            management_protocol: expected_management_protocol.clone(),
        });

        let result = pubsub.create_topic(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        let actual = response.into_inner();
        assert!(Uuid::parse_str(&actual.generated_topic).is_ok());
        assert_eq!(expected_uri, actual.broker_uri);
        assert_eq!(expected_protocol, actual.broker_protocol);

        // This block controls the lifetime of the lock.
        {
            let lock = test_topic_map.lock().unwrap();
            assert!(lock.contains_key(&actual.generated_topic));

            let val = lock.get(&actual.generated_topic);
            assert!(val.is_some());
            let actual_metadata = val.unwrap();
            assert_eq!(expected_metadata.count, actual_metadata.count);
            assert_eq!(
                expected_metadata.management_callback,
                actual_metadata.management_callback,
            );
        }
    }
}
