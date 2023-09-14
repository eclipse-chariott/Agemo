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
    sync::{mpsc, Arc, Mutex},
    time::Instant,
};

use samples_proto::sample_publisher::v1::SubscriptionInfoResponse;

use tonic::Status;

use crate::pub_sub_service_helper::{self, TopicAction};

/// Alias for a map of topics with the relevant metadata.
pub type TopicsMap = HashMap<String, TopicMetadata>;
/// Alias mapping the generated topic to the relevant topic/subject.
pub type GeneratedTopicsMap = HashMap<String, String>;

/// Metadata of a topic.
#[derive(Clone, Debug)]
pub struct TopicMetadata {
    /// The current state of the topic.
    pub action: TopicAction,
    /// The last time the topic had an action taken upon it. Used for topic management.
    pub last_active: Instant,
    /// The relevant subscription information for subscribing to the topic.
    pub subscription_info: SubscriptionInfoResponse,
    /// The channel that is opened when a topic is active.
    pub active_sender: Option<mpsc::Sender<String>>,
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
            active_sender: None,
        }
    }

    /// Deactivates topic, stopping publishing and setting last active timestamp to this instant.
    pub fn deactivate_topic(&mut self) {
        if let Some(sender) = self.active_sender.to_owned() {
            drop(sender);
            self.active_sender = None;
            self.last_active = Instant::now();
        }
    }
}

/// Stores a list of topics with relevant metadata.
#[derive(Clone, Debug, Default)]
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
    pub fn deactivate_topic(&self, topic: &str) {
        if let Some(topic_metadata) = self.topics_map.lock().unwrap().get_mut(topic) {
            topic_metadata.deactivate_topic();
        }
    }

    /// Activates topic, adding the channel for deactivation when the time comes to the topic
    /// metadata. Returns the updated topic metadata.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic to update the `action` field for.
    /// * `sender` - The sender side of an mpsc channel used to stop the publishing thread.
    pub fn activate_topic(
        &self,
        topic: &str,
        sender: mpsc::Sender<String>,
    ) -> Option<TopicMetadata> {
        self.topics_map
            .lock()
            .unwrap()
            .get_mut(topic)
            .map(|topic_metadata| {
                topic_metadata.active_sender = Some(sender);
                topic_metadata.action = TopicAction::Start;
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
