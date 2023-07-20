The samples provide a simple example of a publisher and subscriber interacting with the Pub Sub
Service in a dynamic way.

## Running the simple samples

To run the simple samples, take the following steps.

1. Start the [pub-sub-service](../README.md#running-the-service) in a terminal window.
1. Start the simple publisher in a new terminal window.

    ```shell
    cargo run --bin simple-publisher
    ```

1. Start one or more simple subscribers with a requested subject in a new terminal window.

    ```shell
    cargo run --bin simple-subscriber gps
    ```

You should see simulated data flowing to the subscriber(s).

## Running the Chariott-enabled samples

To run the Chariott samples, take the following steps.

1. Start the [pub-sub-service](../README.md#running-the-service) in a terminal window. The only
   difference is an added Chariott flag when starting the service.

    ```shell
    cargo run --bin pub-sub-service -- --chariott
    ```

1. Start the Chariott publisher in a new terminal window.

    ```shell
    cargo run --bin chariott-publisher
    ```

1. Start one or more Chariott subscribers with a requested subject in a new terminal window.

    ```shell
    cargo run --bin chariott-subscriber gps
    ```

1. At this point all 3 services should be waiting for the Chariott Service to be started up. In a
   new terminal window either pointing to the Chariott repo, or under the 'external/chariott'
   folder, run:

    ```shell
    cargo run -p service_discovery
    ```

All services should then recognize that Chariott has been started:

1. The Pub Sub Service will register with Chariott.
1. The publisher will find the Pub Sub Service through Chariott service discovery and communicate
   with the Pub Sub Service and then register itself with Chariott.
1. The subscriber(s) will find the publisher through Chariott service discovery and use the
   returned endpoint to communicate with the publisher and set up a subscription.

You should then see simulated data flowing to the subscriber(s).

## Understanding the samples

### For all samples

Once the samples are up and running, you can see the dynamic topic management in action several
ways.

1. If you stop the Subscriber with Ctrl+C a STOP message will be sent to the Publisher if there are
   no more subscribers on the topic. Eventually (~30 secs), the Publisher will send a DELETE
   command to the pub-sub-service to remove the dynamic topic.
1. If you stop the Publisher with Ctrl+C while there is a Subscriber on a topic, the Subscriber
   will get a TOPIC DELETED notification on the topic and cleanly disconnect from the broker. Note
   that once the Publisher is stopped, an error will surface if the Chariott service is not stopped
   as this simple example does not unregister itself with Chariott.

In addition, you will see the subject requested (ie. gps) and the dynamically created topic in the
data print outs on the subscriber window.

### For Chariott-enabled samples

The sample subscriber(s) will attempt to find the sample publisher through Chariott service
discovery. If the publisher service is not running yet, then the subscriber will retry discovery
every 5 seconds until the publisher registers.

The sample publisher will attempt to find the Pub Sub Service through Chariott service discovery.
If the Pub Sub Service is not running yet, then the publisher will retry discovery every 5 seconds
until the Pub Sub Service registers.

All services will retry every 5 seconds when attempting connection to Chariott until the Chariott
service is up and running.
