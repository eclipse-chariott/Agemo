# Pub Sub Service Source Code

This folder contains the source code for the Pub Sub Service. The service provides publishing
applications with the ability to dynamically create and manage topics and get messaging broker
information.

## Messaging Broker

The Pub Sub Service was designed with flexibility in mind, allowing for a different messaging
broker to be used, if desired. To use a different messaging broker, a broker connector implementing
the [PubSubConnector](./src/pubsub_connector.rs) trait needs to be created (see the
[mosquitto](./src/connectors/mosquitto_connector.rs) broker connector for an example).

See [Bring Your Own Broker](../docs/README.md#bring-your-own-broker) for a list of requirements.

If a different broker is to be used and it doesn't meet the above requirements, please reach out to
us via a github issue and we can provide assistance with the integration!

## Topic Management

The service provides publisher applications with dynamic topics and management while allowing the
publishers to maintain full control over the lifetime of the created topics.

The service enables this control for publishers in several ways:

### Topic Creation

The service provides a gRPC method `CreateTopic` (see
[pubsub.proto](../proto/pubsub/v1/pubsub.proto)) for publishers to generate a topic to use. The
service returns the generated topic name and message broker connection information. The publisher
can then use this information to start publishing on this created topic.

### Topic Updates

When a publisher requests for a topic to be created, they provide a management callback uri.
This is used by the Pub Sub Service to inform the publisher of events happening on the topic.
Specifically, the service notifies the publisher when the following actions happen:

- **START**: There are zero subscribers on a topic, and a subscribe event occurs.
- **STOP**: There is one subscriber on a topic, and an unsubscribe event occurs.
  > **NOTE**: This is also used if a topic has no subscribers for a period of time and a topic
              still exists. This is planned to be separated out into a TIMEOUT action.

The publisher controls the lifetime of the topic so it is free to ignore these messages. It
provides the publisher with an easy way to determine when to start, stop or delete a dynamically
created topic.

### Topic Deletion

The service provides a gRPC method `DeleteTopic` (see
[pubsub.proto](../proto/pubsub/v1/pubsub.proto)) for publishers to delete a topic that was
generated through the service. This removes the topic from the active topics list and sends a topic
deletion message to all subscribers of the topic, to inform those applications that there will not
be any more messages over that topic.

## Running the Pub Sub Service with Chariott

The service can be run on its own or with
[Eclipse Chariott](https://github.com/eclipse-chariott/chariott). The way the service interacts
with Chariott is through registering itself as a provider that can be discovered through the
Chariott service. Publishers then communicate with Chariott to get connection information to the
service and directly communicate.

### Configure Pub Sub Service to use Chariott

1. Copy the `pub_sub_service_settings.yaml` template to [.agemo/config](../.agemo/config/) if the
file does not already exist. From the enlistment root, run:

    ```shell
    cp ./.agemo/config/template/pub_sub_service_settings.yaml ./.agemo/config/
    ```

2. Uncomment and set the following values:

    ```yaml
    # The URI that the Chariott service listens on for requests.
    # Example: "http://0.0.0.0:50000"
    chariott_uri: "http://0.0.0.0:50000"

    # The namespace of the Pub Sub service.
    # Example: "sdv.pubsub"
    namespace: "sdv.pubsub"

    # The name of the Pub Sub service.
    # Example: "dynamic.pubsub"
    name: "dynamic.pubsub"
    ```

    This will override the default configuration and tell the service to interact with Chariott.
    see [config overrides](../docs/config-overrides.md) for more information.

### Run the Pub Sub Service with Chariott

One can see an example of a publisher and subscriber interacting with Chariott and the Pub Sub
Service in the [Chariott Enabled Samples](../samples/README.md#for-chariott-enabled-samples), but
to simply run the service and have it attempt registration with Chariott, run the following command
in the enlistment's root once the above configuration has been set:

```shell
cargo run -p pub-sub-service
```
