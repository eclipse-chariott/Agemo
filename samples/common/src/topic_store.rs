// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Module that handles the mapping of a topic/subject with the generated topic from the Pub Sub
//! Service.
//!
//! This module handles the mapping between a topic/subject in a publisher to the generated topic
//! given by the Pub Sub Service. The store handles the addition and removal of topics from the
//! topic store.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};

use proto::publisher::v1::SubscriptionInfoResponse;

use tonic::Status;

use crate::pub_sub_service_helper::{self, TopicAction};

/// Alias for a map of topics with the relevant metadata.
pub type TopicsMap = HashMap<String, TopicMetadata>;
/// Alias mapping the generated topic to the relevant topic/subject.
pub type GeneratedTopicsMap = HashMap<String, String>;

/// Metadata of a topic.
#[derive(Clone, Debug, PartialEq)]
pub struct TopicMetadata {
    /// The current state of the topic.
    pub action: TopicAction,
    /// The last time the topic had an action taken upon it. Used for topic management.
    pub last_active: Instant,
    /// The relevant subscription information for subscribing to the topic.
    pub subscription_info: SubscriptionInfoResponse,
}

impl TopicMetadata {
    /// Creates a new instance of the topic metadata based on the relevant subscription info.
    ///
    /// # Arguments
    ///
    /// * `subscription_info` - An object that contains information about how to subscribe to a
    ///                         topic.
    pub fn new(subscription_info: SubscriptionInfoResponse) -> Self {
        TopicMetadata {
            action: TopicAction::Init,
            last_active: Instant::now(),
            subscription_info,
        }
    }

    /// Sets the `last_active` field to the current time.
    pub fn reset_last_active(&mut self) {
        self.last_active = Instant::now();
    }
}

/// Stores a list of topics with relevant metadata.
#[derive(Debug, Default)]
pub struct TopicStore {
    /// Maps topics with the topic metadata.
    topics_map: Arc<Mutex<TopicsMap>>,
    /// Maps the generated topics with the known topics in the `topics_map`.
    generated_topics_map: Arc<Mutex<GeneratedTopicsMap>>,
}

impl TopicStore {
    /// Creates a new instance of the topic store.
    pub fn new() -> Self {
        TopicStore {
            topics_map: Arc::new(Mutex::new(TopicsMap::new())),
            generated_topics_map: Arc::new(Mutex::new(GeneratedTopicsMap::new())),
        }
    }

    /// Adds a topic to the topic store.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to add.
    /// * `subscription_info` - An object that contains information about how to subscribe to a
    ///                         topic.
    pub fn add_topic(&self, topic: String, subscription_info: SubscriptionInfoResponse) {
        let mut topics = self.topics_map.lock().unwrap();
        let mut generated_topics = self.generated_topics_map.lock().unwrap();
        let generated_topic =
            pub_sub_service_helper::get_topic_from_subscription_response(&subscription_info);

        topics.insert(topic.clone(), TopicMetadata::new(subscription_info));
        generated_topics.insert(generated_topic, topic);
    }

    /// Gets the metadata related to the given topic.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to get metadata about.
    pub fn get_topic_metadata(&self, topic: &str) -> Option<TopicMetadata> {
        self.topics_map.lock().unwrap().get(topic).cloned()
    }

    /// Return the associated topic to the publisher since the pub sub service only knows about the
    /// generated topic.
    ///
    /// # Arguments
    ///
    /// * `generated_topic` - The topic that was created by the Pub Sub Service at the request of
    ///                       the publisher.
    pub fn get_generated_topic_mapping(&self, generated_topic: &str) -> Result<String, Status> {
        self.generated_topics_map
            .lock()
            .unwrap()
            .get(generated_topic)
            .map(|topic| (*topic).clone())
            .ok_or_else(|| Status::not_found(generated_topic))
    }

    /// Update last active time field to this instant in stored topic metadata.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to reset the `last_active` time for.
    pub fn set_topic_last_active(&self, topic: &str) {
        if let Some(topic_metadata) = self.topics_map.lock().unwrap().get_mut(topic) {
            topic_metadata.reset_last_active()
        }
    }

    /// Update the topic action field in stored topic metadata.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to update the `action` field for.
    /// * `action` - The action to set the field to.
    pub fn update_topic_action(&self, topic: &str, action: TopicAction) -> Option<TopicMetadata> {
        self.topics_map
            .lock()
            .unwrap()
            .get_mut(topic)
            .map(|topic_metadata| {
                topic_metadata.action = action;
                topic_metadata.last_active = Instant::now();
                topic_metadata.clone()
            })
    }

    /// Removes the topic from the topic store.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to remove.
    /// * `generated_topic` - The generated topic associated with the topic above.
    pub fn remove_topic(&self, topic: &str, generated_topic: &str) {
        let mut topics = self.topics_map.lock().unwrap();
        let mut generated_topics = self.generated_topics_map.lock().unwrap();

        topics.remove(topic);
        generated_topics.remove(generated_topic);
    }
}
