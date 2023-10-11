// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.
// SPDX-License-Identifier: MIT

//! Module handles topic management and informs on the state of a given topic.
//!
//! Handles the logic for dynamic topic management for the pub sub service. Provides a publisher
//! with notifications to allow the publisher to make decisions on a topic that it is publishing to.

use std::{
    collections::{hash_map::Entry::Vacant, HashMap},
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant},
};

use log::{error, info, warn};
use proto::publisher::v1::{
    publisher_callback_client::PublisherCallbackClient, ManageTopicRequest,
};
use tonic::Request;

use crate::{
    load_config::get_uri,
    pubsub_connector::{MonitorMessage, PubSubAction},
};

/// Metadata relevant to a dynamic topic.
#[derive(Clone, Debug, PartialEq)]
pub struct TopicMetadata {
    /// Client id provided by the publisher that will be used to publish from.
    pub client_id: String,
    /// The number of subscribers on a topic.
    pub count: i32,
    deleted: bool,
    last_action: Instant,
    /// Callback uri information for the publisher.
    pub management_callback: Option<String>,
}

impl TopicMetadata {
    /// Creates a new TopicMetadata instance.
    ///
    /// # Arguments
    ///
    /// * `client_id` - The publisher's id.
    /// * `count` - The initial number of subscriptions on the topic.
    /// * `management_cb` - Callback uri for the publisher that created the topic.
    pub fn new(client_id: String, count: i32, management_cb: Option<String>) -> Self {
        TopicMetadata {
            client_id,
            count,
            deleted: false,
            last_action: Instant::now(),
            management_callback: management_cb,
        }
    }

    /// Returns the management callback parameter.
    pub fn get_management_callback(&self) -> Option<String> {
        self.management_callback.clone()
    }

    /// Returns the [`Instant`] of the last action on the topic.
    pub fn get_timeout(&self) -> Instant {
        self.last_action
    }

    /// Returns if the topic is marked for deletion.
    pub fn is_deleted(&self) -> bool {
        self.deleted
    }

    /// Resets the last action to the current [`Instant`].
    pub fn reset_timeout(&mut self) {
        self.last_action = Instant::now();
    }

    /// Marks the topic for deletion.
    pub fn delete(&mut self) {
        self.deleted = true;
    }
}

/// Alias for a HashMap where the key is the topic name as a string
/// and the value is the [`TopicMetadata`].
pub type ActiveTopicsMap = HashMap<String, TopicMetadata>;

/// Associates a topic with the publisher uri that is providing the topic updates.
#[derive(Debug, PartialEq)]
pub struct TopicManagementInfo {
    topic: String,
    uri: String,
}

impl TopicManagementInfo {
    /// Creates a new TopicManagementInfo instance.
    ///
    /// # Arguments
    ///
    /// * `topic` - The topic name.
    /// * `uri` - The management uri for the topic.
    pub fn new(topic: String, uri: String) -> Self {
        TopicManagementInfo { topic, uri }
    }
}

/// Enum that is used to describe an action to take on a topic with the relevant topic information.
#[derive(Debug, PartialEq)]
pub enum TopicAction {
    /// Start enum.
    Start(TopicManagementInfo),
    /// Stop enum.
    Stop(TopicManagementInfo),
    /// Delete enum.
    Delete(TopicManagementInfo),
}

/// Structure that has metadata for a given action on a topic, with a management uri to
/// provide the update to.
pub struct TopicActionMetadata {
    /// Topic that the action is happening to.
    pub topic: String,
    /// Management uri for the publisher.
    pub uri: String,
    /// Action on the topic, represented by [`TopicAction`].
    pub action: String,
}

impl TopicActionMetadata {
    /// Creates a new TopicActionMetadata.
    ///
    /// # Arguments
    ///
    /// * `action` - An action to convert to metadata.
    pub fn new(action: TopicAction) -> Self {
        match action {
            TopicAction::Start(info) => TopicActionMetadata {
                topic: info.topic,
                uri: info.uri,
                action: "START".to_string(),
            },
            TopicAction::Stop(info) => TopicActionMetadata {
                topic: info.topic,
                uri: info.uri,
                action: "STOP".to_string(),
            },
            TopicAction::Delete(info) => TopicActionMetadata {
                topic: info.topic,
                uri: info.uri,
                action: "DELETE".to_string(),
            },
        }
    }
}

/// Handles the management of dynamic topics based on actions on the topic.
///
/// This structure handles the management logic of dynamic topics. It processes actions from the
/// broker connector and from creation and deletion requests from publishers.
pub struct TopicManager {
    active_topics: Arc<Mutex<ActiveTopicsMap>>,
}

impl Default for TopicManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TopicManager {
    /// Instantiates a new TopicManager.
    pub fn new() -> Self {
        let active_topics = Arc::new(Mutex::new(ActiveTopicsMap::new()));

        TopicManager { active_topics }
    }

    /// Returns a handle that points to the active topics list that tracks current known dynamic
    /// topics.
    pub fn get_active_topics_handle(&self) -> Arc<Mutex<ActiveTopicsMap>> {
        self.active_topics.clone()
    }

    /// Updates a topic's metadata based on a [`MonitorMessage`].
    ///
    /// # Arguments
    ///
    /// * `active_topics` - A handle to a shared memory HashMap containing list of topics and
    ///                     associated metadata.
    /// * `msg` - The message that contains information for updating a topic's state.
    fn update_topic(
        active_topics: Arc<Mutex<ActiveTopicsMap>>,
        msg: MonitorMessage,
    ) -> Option<TopicAction> {
        let context = msg.context;
        let action = msg.action;

        let mut map = active_topics.lock().unwrap();

        match action {
            PubSubAction::Subscribe => {
                if let Vacant(m) = map.entry(context.clone()) {
                    // If a subscription happens we want to capture it, but leave the management_cb untouched
                    // so when a suitable publisher comes along it can start publishing.
                    let placeholder_metadata = TopicMetadata::new(String::new(), 1, None);
                    m.insert(placeholder_metadata);
                } else {
                    let mut_val = map.get_mut(&context).unwrap();
                    mut_val.count += 1;
                    mut_val.reset_timeout();

                    // Only want to return an action if there is only one subscriber and there is a publisher to notify.
                    if let Some(management_uri) = mut_val.get_management_callback() {
                        if mut_val.count == 1 {
                            return Some(TopicAction::Start(TopicManagementInfo::new(
                                context.clone(),
                                management_uri,
                            )));
                        }
                    }
                }

                None
            }
            PubSubAction::Unsubscribe => {
                if map.contains_key(&context) {
                    let mut_val = map.get_mut(&context).unwrap();
                    mut_val.count -= 1;
                    mut_val.reset_timeout();

                    // Only want to return an action if there are no longer any subscribers and a publisher to notify.
                    if let Some(management_uri) = mut_val.get_management_callback() {
                        if mut_val.count <= 0 {
                            mut_val.count = 0; // Potential edge case with duplicate messages causing count to go below zero

                            return Some(TopicAction::Stop(TopicManagementInfo::new(
                                context.clone(),
                                management_uri,
                            )));
                        }
                    }
                }

                None
            }
            PubSubAction::Timeout => {
                if map.contains_key(&context) {
                    let mut_val = map.get_mut(&context).unwrap();
                    mut_val.reset_timeout();

                    // Only want to return an action if there is a publisher to notify.
                    if let Some(management_uri) = mut_val.get_management_callback() {
                        if mut_val.count <= 0 {
                            mut_val.count = 0; // Potential edge case with duplicate messages causing count to go below zero

                            return Some(TopicAction::Stop(TopicManagementInfo::new(
                                context.clone(),
                                management_uri,
                            )));
                        }
                    }
                }

                None
            }
            PubSubAction::Delete => map
                .remove(&context)
                .and_then(|metadata| metadata.get_management_callback())
                .map(|management_uri| {
                    TopicAction::Delete(TopicManagementInfo::new(context, management_uri))
                }),
            _ => {
                warn!("Shouldn't be here! Invalid action: {action}");
                None
            }
        }
    }

    /// Notifies a publisher of the given action on a topic.
    ///
    /// # Arguments
    ///
    /// * `action` - The specific action to be taken on a topic.
    async fn manage_topic(
        action: TopicAction,
    ) -> Result<TopicActionMetadata, Box<dyn std::error::Error + Send + Sync>> {
        // Get action details
        let action_metadata = TopicActionMetadata::new(action);
        info!(
            "Executing action '{}' on topic '{}'.",
            action_metadata.action, action_metadata.topic
        );

        // No need to contact publisher if DELETE action as this is initated by the publisher.
        if action_metadata.action == PubSubAction::Delete.to_string() {
            return Ok(action_metadata);
        }

        // Get information from publisher client
        let uri = get_uri(&action_metadata.uri)?;
        let mut pub_client = PublisherCallbackClient::connect(uri).await?;

        let request = Request::new(ManageTopicRequest {
            topic: action_metadata.topic.clone(),
            action: action_metadata.action.clone(),
        });

        let _response = pub_client.manage_topic_callback(request).await?;

        Ok(action_metadata)
    }

    /// Internal function that periodically handles deletion of inactive topics.
    ///
    /// # Arguments
    ///
    /// * `active_topics_handle` - A handle to a shared memory HashMap containing list of topics
    ///                            and associated metadata.
    /// * `drop_sender` - The sender used to communicate a delete action request.
    async fn cleanup_topics(
        active_topics_handle: Arc<Mutex<ActiveTopicsMap>>,
        drop_sender: mpsc::Sender<MonitorMessage>,
    ) {
        let active_topics = active_topics_handle.lock().unwrap();

        let threshold = Duration::from_secs(30);

        for (topic, metadata) in active_topics.clone().into_iter() {
            if metadata.is_deleted() {
                // If the topic has been marked for deletion, then send a deletion action and move to the next topic.
                info!("Removed topic '{topic}' as it is no longer being used.");
                let _ = drop_sender.send(MonitorMessage {
                    context: topic,
                    action: PubSubAction::Delete,
                });
            } else if metadata.count == 0
                && metadata.get_timeout().elapsed().as_secs() > threshold.as_secs()
            {
                // If count is 0 and the time since the last action is greater than the threshold, then notify to remove from list.
                info!("Topic '{topic}' hit a timeout, reminding publisher.");
                let _ = drop_sender.send(MonitorMessage {
                    context: topic,
                    action: PubSubAction::Timeout,
                });
            }
        }
    }

    /// Processes a given [`MonitorMessage`] and updates topic state.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message that contains information for updating a topic's state.
    /// * `active_topics_handle` - A handle to a shared memory HashMap containing list of topics
    ///                            and associated metadata.
    /// * `deletion_ch` - A channel used to handle a delete action from the publisher.
    pub async fn handle_topic_action(
        msg: MonitorMessage,
        active_topics_handle: Arc<Mutex<ActiveTopicsMap>>,
        deletion_ch: mpsc::Sender<MonitorMessage>,
    ) {
        if let Some(action) = Self::update_topic(active_topics_handle.clone(), msg) {
            let result = Self::manage_topic(action).await;

            match result {
                Ok(action) => {
                    if action.action == PubSubAction::Delete.to_string() {
                        let _res = deletion_ch.send(MonitorMessage {
                            context: action.topic,
                            action: PubSubAction::Delete,
                        });
                    }
                }
                Err(err) => {
                    error!("error executing action: {err}");
                }
            }
        }
    }

    /// Continuously monitors a channel where updates to topics are sent as MonitorMessages.
    ///
    /// # Arguments
    ///
    /// * `deletion_ch` - A channel used to handle a delete action from the publisher.
    pub async fn monitor(
        &self,
        deletion_ch: mpsc::Sender<MonitorMessage>,
    ) -> mpsc::Sender<MonitorMessage> {
        let (sender, receiver) = mpsc::channel::<MonitorMessage>();

        let active_topics_handle = self.get_active_topics_handle();

        let drop_sender = sender.clone();

        let _monitor_handle = tokio::spawn(async move {
            loop {
                let update = receiver.recv();

                match update {
                    Ok(msg) => {
                        // Check if the action was a disconnect, if so we need to gather the topics to clean up.
                        if msg.action == PubSubAction::PubDisconnect {
                            let mut topics_to_notify = Vec::<String>::new();
                            info!("{} publisher disconnected", &msg.context);

                            // Gets the list of topics to send Delete messages to.
                            {
                                let map = active_topics_handle.lock().unwrap();

                                for (topic, metadata) in map.clone().into_iter() {
                                    if metadata.client_id == msg.context {
                                        topics_to_notify.push(topic.clone());
                                    }
                                }
                            }

                            for topic in topics_to_notify {
                                // for each topic, execute a DELETE action as the publisher is disconnected and won't publish again.
                                let topic_action = MonitorMessage {
                                    context: topic.clone(),
                                    action: PubSubAction::Delete,
                                };

                                // Clone sender for the deletion channel callback.
                                let deletion_channel = deletion_ch.clone();

                                Self::handle_topic_action(
                                    topic_action,
                                    active_topics_handle.clone(),
                                    deletion_channel,
                                )
                                .await;
                            }
                        } else {
                            // Clone sender for the deletion channel callback.
                            let deletion_channel = deletion_ch.clone();

                            Self::handle_topic_action(
                                msg,
                                active_topics_handle.clone(),
                                deletion_channel,
                            )
                            .await;
                        }
                    }
                    Err(err) => {
                        error!("error from monitor: {err}");
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        });

        let active_topics_handle = self.get_active_topics_handle();

        let _cleanup_handle = tokio::spawn(async move {
            loop {
                let drop_sender = drop_sender.clone();
                Self::cleanup_topics(active_topics_handle.clone(), drop_sender).await;

                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        sender
    }
}

#[cfg(test)]
mod topic_action_tests {
    use super::*;

    #[tokio::test]
    async fn initialize_topic_action_metadata_test() {
        let expected_topic = "test".to_string();
        let expected_mgmt_uri = "test.uri".to_string();
        let expected_start_action = "START".to_string();
        let expected_stop_action = "STOP".to_string();
        let expected_delete_action = "DELETE".to_string();
        let start_action = TopicAction::Start(TopicManagementInfo::new(
            expected_topic.clone(),
            expected_mgmt_uri.clone(),
        ));

        let start_action_metadata = TopicActionMetadata::new(start_action);

        assert_eq!(expected_topic, start_action_metadata.topic);
        assert_eq!(expected_mgmt_uri, start_action_metadata.uri);
        assert_eq!(expected_start_action, start_action_metadata.action);

        let stop_action = TopicAction::Stop(TopicManagementInfo::new(
            expected_topic.clone(),
            expected_mgmt_uri.clone(),
        ));

        let stop_action_metadata = TopicActionMetadata::new(stop_action);

        assert_eq!(expected_topic, stop_action_metadata.topic);
        assert_eq!(expected_mgmt_uri, stop_action_metadata.uri);
        assert_eq!(expected_stop_action, stop_action_metadata.action);

        let delete_action = TopicAction::Delete(TopicManagementInfo::new(
            expected_topic.clone(),
            expected_mgmt_uri.clone(),
        ));

        let delete_action_metadata = TopicActionMetadata::new(delete_action);

        assert_eq!(expected_topic, delete_action_metadata.topic);
        assert_eq!(expected_mgmt_uri, delete_action_metadata.uri);
        assert_eq!(expected_delete_action, delete_action_metadata.action);
    }
}

#[cfg(test)]
mod topic_manager_tests {
    use super::*;

    #[tokio::test]
    async fn subscribe_topic_test() {
        let test_manager = TopicManager::new();
        let topic_map_handle = test_manager.get_active_topics_handle();
        let expected_topic = "test".to_string();
        let initial_count = 1;
        let expected_mgmt_uri = "test.uri".to_string();
        let initial_metadata =
            TopicMetadata::new(String::new(), initial_count, Some(expected_mgmt_uri));
        let initial_time = initial_metadata.get_timeout();

        // Insert existing topic
        {
            let mut map_lock = topic_map_handle.lock().unwrap();
            map_lock.insert(expected_topic.clone(), initial_metadata);
        }

        let message = MonitorMessage {
            context: expected_topic.clone(),
            action: PubSubAction::Subscribe,
        };

        let actual_action = TopicManager::update_topic(topic_map_handle.clone(), message);
        assert!(actual_action.is_none());

        // Confirm last active time and count was updated
        {
            let map_lock = topic_map_handle.lock().unwrap();
            let actual_metadata = map_lock.get(&expected_topic).unwrap();

            assert_ne!(initial_time, actual_metadata.get_timeout());
            assert_eq!(initial_count + 1, actual_metadata.count);
        }
    }

    #[tokio::test]
    async fn subscribe_topic_with_no_subs_test() {
        let test_manager = TopicManager::new();
        let topic_map_handle = test_manager.get_active_topics_handle();
        let expected_topic = "test".to_string();
        let initial_count = 0;
        let expected_mgmt_uri = "test.uri".to_string();
        let initial_metadata = TopicMetadata::new(
            String::new(),
            initial_count,
            Some(expected_mgmt_uri.clone()),
        );
        let initial_time = initial_metadata.get_timeout();

        // Insert existing topic with no active subs
        {
            let mut map_lock = topic_map_handle.lock().unwrap();
            map_lock.insert(expected_topic.clone(), initial_metadata);
        }

        let message = MonitorMessage {
            context: expected_topic.clone(),
            action: PubSubAction::Subscribe,
        };

        let actual_action = TopicManager::update_topic(topic_map_handle.clone(), message);
        assert!(actual_action.is_some());

        let expected_action_inner = TopicAction::Start(TopicManagementInfo::new(
            expected_topic.clone(),
            expected_mgmt_uri,
        ));
        assert_eq!(expected_action_inner, actual_action.unwrap());

        // Confirm last active time and count was updated
        {
            let map_lock = topic_map_handle.lock().unwrap();
            let actual_metadata = map_lock.get(&expected_topic).unwrap();

            assert_ne!(initial_time, actual_metadata.get_timeout());
            assert_eq!(initial_count + 1, actual_metadata.count);
        }
    }

    #[tokio::test]
    async fn subscribe_topic_with_no_active_topic_test() {
        let test_manager = TopicManager::new();
        let topic_map_handle = test_manager.get_active_topics_handle();
        let expected_topic = "test".to_string();
        let initial_count = 1;
        let expected_metadata = TopicMetadata::new(String::new(), initial_count, None);

        let message = MonitorMessage {
            context: expected_topic.clone(),
            action: PubSubAction::Subscribe,
        };

        let actual_action = TopicManager::update_topic(topic_map_handle.clone(), message);
        assert!(actual_action.is_none());

        // Confirm metadata matches expected
        {
            let map_lock = topic_map_handle.lock().unwrap();
            let actual_metadata = map_lock.get(&expected_topic).unwrap();

            assert_eq!(expected_metadata.count, actual_metadata.count);
            assert_eq!(
                expected_metadata.management_callback,
                actual_metadata.management_callback,
            );
        }
    }

    #[tokio::test]
    async fn unsubscribe_topic_test() {
        let test_manager = TopicManager::new();
        let topic_map_handle = test_manager.get_active_topics_handle();
        let expected_topic = "test".to_string();
        let initial_count = 2;
        let expected_mgmt_uri = "test.uri".to_string();
        let initial_metadata =
            TopicMetadata::new(String::new(), initial_count, Some(expected_mgmt_uri));
        let initial_time = initial_metadata.get_timeout();

        // Insert existing topic
        {
            let mut map_lock = topic_map_handle.lock().unwrap();
            map_lock.insert(expected_topic.clone(), initial_metadata);
        }

        let message = MonitorMessage {
            context: expected_topic.clone(),
            action: PubSubAction::Unsubscribe,
        };

        let actual_action = TopicManager::update_topic(topic_map_handle.clone(), message);
        assert!(actual_action.is_none());

        // Confirm last active time and count was updated
        {
            let map_lock = topic_map_handle.lock().unwrap();
            let actual_metadata = map_lock.get(&expected_topic).unwrap();

            assert_ne!(initial_time, actual_metadata.get_timeout());
            assert_eq!(initial_count - 1, actual_metadata.count);
        }
    }

    #[tokio::test]
    async fn unsubscribe_topic_with_one_sub_test() {
        let test_manager = TopicManager::new();
        let topic_map_handle = test_manager.get_active_topics_handle();
        let expected_topic = "test".to_string();
        let initial_count = 1;
        let expected_mgmt_uri = "test.uri".to_string();
        let initial_metadata = TopicMetadata::new(
            String::new(),
            initial_count,
            Some(expected_mgmt_uri.clone()),
        );
        let initial_time = initial_metadata.get_timeout();

        // Insert existing topic
        {
            let mut map_lock = topic_map_handle.lock().unwrap();
            map_lock.insert(expected_topic.clone(), initial_metadata);
        }

        let message = MonitorMessage {
            context: expected_topic.clone(),
            action: PubSubAction::Unsubscribe,
        };

        let actual_action = TopicManager::update_topic(topic_map_handle.clone(), message);
        assert!(actual_action.is_some());

        let expected_action_inner = TopicAction::Stop(TopicManagementInfo::new(
            expected_topic.clone(),
            expected_mgmt_uri,
        ));
        assert_eq!(expected_action_inner, actual_action.unwrap());

        // Confirm last active time and count was updated
        {
            let map_lock = topic_map_handle.lock().unwrap();
            let actual_metadata = map_lock.get(&expected_topic).unwrap();

            assert_ne!(initial_time, actual_metadata.get_timeout());
            assert_eq!(initial_count - 1, actual_metadata.count);
        }
    }

    #[tokio::test]
    async fn unsubscribe_topic_with_no_subs_test() {
        let test_manager = TopicManager::new();
        let topic_map_handle = test_manager.get_active_topics_handle();
        let expected_topic = "test".to_string();
        let initial_count = 0;
        let expected_mgmt_uri = "test.uri".to_string();
        let initial_metadata = TopicMetadata::new(
            String::new(),
            initial_count,
            Some(expected_mgmt_uri.clone()),
        );
        let initial_time = initial_metadata.get_timeout();

        // Insert existing topic
        {
            let mut map_lock = topic_map_handle.lock().unwrap();
            map_lock.insert(expected_topic.clone(), initial_metadata);
        }

        let message = MonitorMessage {
            context: expected_topic.clone(),
            action: PubSubAction::Unsubscribe,
        };

        let actual_action = TopicManager::update_topic(topic_map_handle.clone(), message);
        assert!(actual_action.is_some());

        let expected_action_inner = TopicAction::Stop(TopicManagementInfo::new(
            expected_topic.clone(),
            expected_mgmt_uri,
        ));
        assert_eq!(expected_action_inner, actual_action.unwrap());

        // Confirm last active time and count was updated
        {
            let map_lock = topic_map_handle.lock().unwrap();
            let actual_metadata = map_lock.get(&expected_topic).unwrap();

            assert_ne!(initial_time, actual_metadata.get_timeout());
            assert_eq!(initial_count, actual_metadata.count);
        }
    }

    #[tokio::test]
    async fn manage_topic_on_delete_action() {
        let delete_action = TopicAction::Delete(TopicManagementInfo::new(
            "topic".to_string(),
            "uri".to_string(),
        ));

        let ok_result = TopicManager::manage_topic(delete_action).await;

        // Expect that result is short circuited to ok. Since the Publisher connector is
        // not mocked it will return an error if action does not match Delete.
        assert!(ok_result.is_ok());
    }
}
