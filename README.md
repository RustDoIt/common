# Overview of the `common` library

The `common` library provides foundational components for a drone-based network simulation in Rust, supporting packet routing, fragmentation, reassembly, and node management in an unreliable network environment. It integrates with `wg_internal` for core packet and node primitives. Key design principles include idempotent operations, flood-based discovery, source routing, and resilience to packet drops or node crashes via acknowledgments and retries.

## Modules and Key Components

### `types`
Defines core data structures and enums for network entities, files, requests/responses, commands, and events.

- **MediaReference**: Represents a reference to media stored at a specific node (NodeId) with a UUID.
- **TextFile**: Encapsulates a text file with title, content, and embedded media references.
- **MediaFile**: Handles binary media files, chunked into 1024-byte segments for transmission.
- **File**: Composite of a TextFile and associated MediaFiles.
- **WebRequest/WebResponse**: Enums for web-like queries (e.g., server type, file lists, media retrieval) and responses (e.g., data delivery, errors like not found or UUID parsing failures).
- **ChatRequest/ChatResponse**: Enums for chat operations (e.g., registration, client lists, messaging) and responses (e.g., message delivery, client lists).
- **Event/Command**: Traits and enums for node-specific events (e.g., NodeEvent for packet sent/flood started) and commands (e.g., NodeCommand for adding/removing senders, shutdown).
- **ChatEvent/WebEvent/NodeEvent**: Specific event variants for chat (e.g., message received, registration), web (e.g., file added/removed, queries), and general node operations.
- **ClientType/ServerType/NodeType**: Enums classifying nodes (e.g., ChatClient, TextServer, Drone).

### `assembler`
Manages packet fragmentation and reassembly.

- **FragmentAssembler**: Tracks fragments by session ID and sender NodeId. Adds fragments, checks completeness via expected/received counts, and reassembles data into a complete message when all fragments arrive.

### `file_conversion`
Utilities for converting local files to library types.

- **file_to_media_file**: Reads binary file content, chunks it, and creates a MediaFile.
- **file_to_text_file**: Reads text file content and creates a TextFile (without media refs by default).

### `network`
Models the network topology and operations.

- **NetworkError**: Enum for errors like path not found, node removal, or send failures.
- **Node**: Represents a network node with ID, type (NodeType), and adjacent nodes.
- **Network**: Maintains a list of nodes; supports adding/removing/updating nodes, changing types, finding shortest paths via BFS, and filtering by type (e.g., get_servers, get_clients).

### `routing_handler`
Handles routing logic, including discovery and packet transmission.

- **RoutingHandler**: Core struct managing node ID, network view (Network), neighbors (senders by NodeId), flood tracking, and buffers for packets/fragments.
    - Initiates floods for discovery (start_flood).
    - Handles flood requests/responses to update topology.
    - Sends messages with fragmentation if >128 bytes (send_message).
    - Processes acks (mark fragments received), nacks (retry or remove faulty nodes), and retries (retry_send).
    - Manages neighbor addition/removal and buffering for pending packets.

### `packet_processor`
Defines processing loop for packets and commands.

- **Processor**: Trait for entities that handle incoming packets and commands.
    - Integrates FragmentAssembler and RoutingHandler.
    - Processes packets (e.g., fragments to reassemble messages, acks/nacks/floods via routing handler).
    - Runs an event loop selecting between controller commands (handle_command) and packets (handle_packet), with flood initiation on start.
    - Subtypes must implement message handling (handle_msg) and command processing.