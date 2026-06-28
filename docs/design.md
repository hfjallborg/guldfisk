# Connections

A single thread will listen for connections on a given port using TCP. When a new connection is received,
a thread will be spawned to handle that specific connection.

The per-connection threads should accept instructions from the client and add them to a queue.

## Connection lifecycle

1. A client connects to the server.
2. A thread is spawned to handle the connection.
3. The thread clones the shared channel sender
4. The thread listens for instructions from the client.
5. When an instruction is received, it is added to a shared queue for execution.
6. The thread continues to listen for more instructions until the client disconnects.
7. When the client disconnects, the thread is terminated.

# Executing instructions

| Description          | Returns                                              |
|----------------------|------------------------------------------------------|
| Set a key-value pair | Ok()                                                 |
| Get a value by key   | Return(value) if key exists, else Error(KeyNotFound) |
| Delete a key         | Ok(), regardless of whether the key existed or not   |

A single thread will be responsible for executing instructions from *all* connections. This is similar to how
Redis handles commands. This will require a thread-safe queue that can be shared between all connection threads.

The command queue will be implemented using `crossbeam::channel` to allow for multiple producers (connection threads)
and a single consumer (execution thread).

## Execution thread lifecycle

1. The thread initializes the HashMap cache
2. The execution thread creates a channel to receive instructions from connection threads.

# Cache

The cache will be built using a hash map to store key-value pairs. There should exist commands
to set, get, and delete pairs from the cache. When setting a key-value pair, an optional expiration time can be
provided. If the expiration time is provided, the key-value pair will be automatically deleted from the cache after the
specified time has elapsed.

## Hash map

- `foldhash`

## Expiration

Expiration could be handled in multiple ways. One approach is to check the expiry time when a key is accessed, and
delete it if it has expired. Another approach is to have a separate thread that periodically checks for expired keys and
deletes them from the cache.

# Message broker

A client should be able to subscribe to named channels. When a client publishes a message to a channel, the connection
threads that are subscribed to that channel should receive the message. 