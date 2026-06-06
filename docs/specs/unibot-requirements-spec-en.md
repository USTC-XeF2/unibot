# UniBot Requirements Specification

---

## 0. Document Information

### 0.1 Purpose

This document is the Software Requirements Specification (SRS) for the UniBot project. Its purposes are:

1. To define the functional requirements, non-functional requirements, and constraints of the UniBot system
2. To provide a unified requirements baseline for subsequent database design, architecture design, implementation, and testing
3. To serve as the authoritative reference for open-source contributors and reviewers

### 0.2 Scope

This document covers all requirements for the UniBot system, spanning five core domains: Bot debugging and management, protocol packet tracing, messaging and conversation management, social data mirroring, and audit & system maintenance. Installation and operational guides are out of scope.

> **Note**: Field names and entity names in this document express requirement constraints and business semantics; they do not represent the final database table structure. The final table structure is defined in the database design document.

### 0.3 Intended Audience

| Audience | Purpose |
|----------|---------|
| Project developers | Understand what the system must do; guide design and coding |
| Open-source contributors | Quickly understand project positioning, boundaries, and core requirements |
| Protocol adapter developers | Understand UniBot's expectations and interface constraints for external protocol endpoints |

### 0.4 Glossary

| Term | Definition |
|------|------------|
| **UniBot** | The system: a multi-Bot local debugging and management platform for QQ Bots |
| **Bot** | A bot instance registered in UniBot, bound to one IM Account, driven by a protocol adapter |
| **IM Account** | An external IM platform account, primarily a QQ account in this document's context; may originate from a simulated or real protocol endpoint |
| **Adapter** | The protocol adaptation layer that normalizes external protocol (Milky / OneBot-v11 / OneBot-v12) events and APIs into UniBot's internal model |
| **Protocol Packet** | A complete record of one protocol interaction, containing metadata (action, direction, timestamp) and the raw JSON payload |
| **Debug Session** | A Bot run cycle from start to stop, used to group messages and protocol packets from the same debugging session |
| **Conversation** | A user-facing message container abstracting three chat scenes: private, group, and temporary |
| **Message Scene** | The scene type of a message: `private`, `group`, or `temp` |
| **Peer ID** | The peer identifier of a conversation: the group ID for group chats, or the other user's ID for private/temp chats |
| **Message Seq** | A monotonically increasing sequence number for messages within a specific scene + peer, maintained by the protocol endpoint |
| **Group** | A QQ group, containing sub-entities such as members, announcements, files, and albums |
| **Friend** | A QQ friendship relationship, with attributes including remark name, category, and pin status |
| **Audit Event** | A system-level audit record of who performed what action on what entity at what time |
| **Simulated / Real** | Environment distinction: `simulated` originates from UniBot's built-in virtual server; `real` originates from an external QQ protocol endpoint |
| **Milky** | A QQ bot application interface standard using HTTP/WebSocket, providing message, event, and API interfaces |
| **OneBot** | A universal chat bot application interface standard; versions v11 and v12 differ in event models and API design |
| **Tauri** | A cross-platform desktop application framework using a Rust backend and web frontend |
| **SQLite** | An embedded relational database engine; UniBot's local persistence layer |

### 0.5 Requirement Numbering Scheme

| Prefix | Meaning | Example |
|--------|---------|---------|
| `FR-ACC-` | Functional — Account Management | FR-ACC-001 |
| `FR-BOT-` | Functional — Bot Instance Management | FR-BOT-001 |
| `FR-MSG-` | Functional — Conversations & Messages | FR-MSG-001 |
| `FR-SOC-` | Functional — Friends, Groups & Members | FR-SOC-001 |
| `FR-REQ-` | Functional — Requests & Events | FR-REQ-001 |
| `FR-PKT-` | Functional — Protocol Packet Tracing | FR-PKT-001 |
| `FR-DBG-` | Functional — Debug Session Management | FR-DBG-001 |
| `FR-CFG-` | Functional — Configuration Management | FR-CFG-001 |
| `FR-AUD-` | Functional — Audit, Export & Maintenance | FR-AUD-001 |
| `DR-` | Data Requirement | DR-CLN-001 |
| `NFR-PERF-` | Non-Functional — Performance | NFR-PERF-001 |
| `NFR-CAP-` | Non-Functional — Capacity | NFR-CAP-001 |
| `NFR-REL-` | Non-Functional — Reliability | NFR-REL-001 |
| `NFR-SEC-` | Non-Functional — Security & Privacy | NFR-SEC-001 |
| `NFR-MNT-` | Non-Functional — Maintainability | NFR-MNT-001 |
| `NFR-OBS-` | Non-Functional — Observability | NFR-OBS-001 |
| `NFR-CMP-` | Non-Functional — Compatibility | NFR-CMP-001 |
| `OUT-` | Out of Scope | OUT-001 |
| `S-DEV-` | Scenario — Bot Developer | S-DEV-001 |
| `S-ADM-` | Scenario — Bot Administrator | S-ADM-001 |
| `S-DBG-` | Scenario — Protocol Debugger | S-DBG-001 |
| `S-MNT-` | Scenario — System Maintainer | S-MNT-001 |
| `ASM-` | Assumption | ASM-001 |
| `DEP-` | External Dependency | DEP-001 |

Each functional requirement follows this template:

```
ID:          FR-XXX-NNN
Name:        Short requirement name
Description: The system shall... (main flow)
Priority:    P0 / P1 / P2
Roles:       R1-R4
Scenarios:   S-XXX-NNN
Precondition:  State before triggering
Postcondition: State after fulfillment
Acceptance:    Verifiable conditions
Data Domain:   Associated data domain
```

---

## 1. Project Background

### 1.1 Pain Points in QQ Bot Development & Debugging

QQ Bot developers face the following challenges:

1. **Protocol Black Box**: Bots communicate with QQ servers via protocols like Milky or OneBot. Protocol requests and responses are opaque to developers, making it difficult to diagnose issues like "message not delivered" or "event not received."
2. **Lack of Structured Debugging Tools**: Developers typically rely on terminal logs or packet captures, lacking a structured interface that organizes protocol packets by Bot, conversation, and time.
3. **Multi-Bot Management Chaos**: A single developer may maintain multiple Bot instances, each bound to different QQ accounts, using different configurations, and facing different groups. A unified management interface is missing.
4. **No Message-to-Protocol Correlation**: When a user reports that "the Bot didn't reply to a message," the developer must manually correlate: "which protocol event corresponds to that message," "did the Bot receive the event," and "did the Bot's API call succeed."
5. **Simulated Testing Difficulty**: Developers need to test Bot behavior without affecting the real QQ environment, but lack a built-in simulated IM environment.

### 1.2 Target Users

UniBot is designed for:

- **QQ Bot Developers**: Individuals or teams developing QQ bots using Milky / OneBot protocols, who need to debug the correctness of message handling, event processing, and API calls.
- **Bot Operators**: Those who need to manage connection states, configuration versions, and group behavior policies across multiple Bot instances.

### 1.3 Core Value Proposition

> **UniBot is a multi-Bot local debugging and management platform for QQ Bots.** By selectively mirroring QQ data relevant to Bot debugging (messages, conversations, groups, friend profiles, and key protocol events), it provides developers with full-chain traceability: from "the message a user sees" to "the protocol event the Bot received" to "the API call the Bot made." All data is stored entirely locally, protecting privacy and security.

---

## 2. Project Positioning

### 2.1 System Positioning

UniBot is positioned as a **multi-Bot local debugging and management platform**, delivered as a Tauri-based desktop application.

Core data flow:

```
External Protocol Endpoint (Milky / OneBot)
        ↓ events & API responses
UniBot Adapter Layer (normalization)
        ↓
Local Database + File System (full storage)
        ↓
React Frontend (Bot management / Conversations / Protocol tracing / Audit)
```

UniBot is NOT:

- A production-grade cloud Bot hosting platform
- A complete QQ client replacement
- A long-term chat archive system

### 2.2 Core Capabilities

| ID | Capability | Description |
|----|------------|-------------|
| C1 | **Multi-Bot Management** | Register, start, and stop multiple Bot instances; manage Bot-to-IM-Account bindings |
| C2 | **Conversation & Message Management** | Organize messages by conversation; support private/group/temp scenes; maintain unread state and conversation list |
| C3 | **Debug Context Mirroring** | Cache friends, groups, group members, and low-frequency data returned by the protocol endpoint as needed for debugging, to support message display, Bot behavior debugging, and protocol tracing |
| C4 | **Protocol Packet Tracing** | Record metadata and raw JSON for every protocol interaction; filter by Bot, time, error status |
| C5 | **Debug Session Aggregation** | Group messages and protocol packets from one Bot run cycle into reviewable debug sessions |
| C6 | **Configuration Management** | Manage Bot connection and behavior configurations; record configuration change history |
| C7 | **Audit & Security** | Record all critical operations; support audit tracing and data cleanup |

### 2.3 Non-Goals Overview

UniBot explicitly does NOT aim to:

1. Be a complete QQ client replacement — it does not guarantee syncing all historical messages
2. Provide multi-device multi-user collaboration
3. Serve as a production cloud-hosted Bot platform
4. Directly manage QQ login credentials (delegated to the protocol endpoint)
5. Guarantee field-level consistency across different protocol endpoints

See Chapter 10 for the formal out-of-scope item list.

---

## 3. Constraints & Context

### 3.1 External Protocol Constraints

UniBot's core functionality depends on events and APIs provided by external protocol endpoints. The system shall support:

| Protocol | Version | Transport | Notes |
|----------|---------|-----------|-------|
| Milky | Current | HTTP / WebSocket | QQ bot application interface standard; message and event APIs with QQ-specific features |
| OneBot | v11 | HTTP / WebSocket / Webhook | Universal chat bot interface standard; traditional event model |
| OneBot | v12 | HTTP / WebSocket | Standardized event structures and APIs; differs from v11 |

Key protocol constraints:

- Message location relies on the `message_scene` + `peer_id` + `message_seq` triple (Milky definition). UniBot's internal model uses `normalized_scene + normalized_peer_id + normalized_message_seq`. For Milky, these map directly to `message_scene + peer_id + message_seq`; for OneBot-v11/v12, the Adapter layer converts or generates equivalent positioning fields. If a protocol endpoint cannot provide a stable seq, the system shall use the endpoint's `message_id` or a locally generated `normalized_message_id` as a fallback unique identifier
- Message retrieval is inherently paginated (Milky defaults to 20 per page, max 30)
- Field naming, event types, and error code systems differ across protocols; the Adapter layer normalizes these
- Some QQ entity queries (e.g., group files, group albums) may not be fully supported by all protocol endpoints; UniBot only stores what the endpoint returns

### 3.2 Runtime Environment & Deployment

| Item | Constraint |
|------|------------|
| Operating System | Windows 10+ / macOS 12+ / Linux (major distributions) |
| Deployment | Single-machine desktop application; one installation = one local database |
| Network | Only needs to communicate with locally or LAN-deployed protocol endpoints; no internet access required (the endpoint itself may need it) |
| Hardware | 2 GB+ RAM, 100 MB+ disk space (excluding protocol packet file storage) |

### 3.3 Technology Stack

| Layer | Technology |
|-------|------------|
| Desktop Framework | Tauri 2.x (Rust) |
| Frontend | React 19 + TypeScript |
| Database | SQLite (via Tauri plugin) |
| Bulk File Storage | Local file system (JSON files) |
| UI Components | shadcn/ui + Tailwind CSS 4 |
| Build Tooling | Vite 7 |

Core principle: **fully local** — all data stored locally, no network service required; **minimal resource footprint** — a desktop application should not consume excessive memory or CPU.

### 3.4 Assumptions & Dependencies

**Assumptions:**

| ID | Assumption |
|----|------------|
| ASM-001 | At least one usable QQ protocol adapter endpoint (Milky or OneBot implementation) exists and can communicate with UniBot via HTTP/WebSocket |
| ASM-002 | The local runtime environment has file system read/write permissions and sufficient disk space for protocol packet files |
| ASM-003 | The protocol endpoint can provide a uniquely identifying field combination for each message (at minimum: scene + peer_id + seq) |
| ASM-004 | The user operates UniBot on a single computer; no concurrent multi-user database access occurs |
| ASM-005 | The `simulated` vs `real` environment distinction is explicitly provided by the user or protocol endpoint; UniBot does not auto-infer |

**External Dependencies:**

| ID | Dependency | Impact |
|----|------------|--------|
| DEP-001 | External protocol endpoints provide message events, API responses, and account/group/friend profile queries | If an endpoint does not provide a query interface, the corresponding mirroring capability is limited |
| DEP-002 | Local SQLite database and file system jointly provide persistence | Database or file system corruption may cause data loss |
| DEP-003 | The protocol endpoint maintains `message_seq` monotonicity | If seq is non-monotonic, message ordering and deduplication logic degrades |
| DEP-004 | Tauri runtime's WebView component | WebView version differences across operating systems may affect UI rendering |

### 3.5 External Interface Requirements Overview

**3.5.1 Protocol Adapter Interface**

The system shall receive the following data types from protocol adapters (Milky / OneBot-v11 / OneBot-v12):

- **Message events**: new message notifications, containing message content, sender, and conversation identifiers
- **API responses**: the endpoint's response to API calls issued by the Bot
- **Status events**: Bot connection state, group member changes, friend requests, and other business events
- **Profile query responses**: results of queries for friend profiles, group profiles, group member lists, etc.

The system shall record the request-response correlation for each protocol interaction. When protocol endpoint fields are missing, the system shall preserve the raw data and mark the adaptation status.

**3.5.2 Local File System Interface**

The system needs to read and write:

- Bot configuration files (JSON format, stored under `configs/bots/`)
- Protocol packet raw payload files (JSON format, stored under `data/packets/`)
- Application log files
- Cache files (avatars, custom faces, etc.)

The system shall read and write local files. For protocol packet raw JSON, the system shall lazily check file existence when viewing or exporting via `file_path`, and show a clear error when the file is missing or unreadable.

**3.5.3 Database Interface Boundary**

The SQLite database is accessed through the Tauri SQL plugin. The database stores only structured index fields and business relationships; large JSON content (protocol packet payloads, message segment arrays) follows a "database stores index + file system stores content" strategy.

---

## 4. User Roles

| ID | Role | Responsibilities | Primary Concerns |
|----|------|------------------|------------------|
| **R1** | **Bot Developer** | Develops and debugs Bot message handling logic, API calls, and event responses | Are messages sent/received correctly? Did the API call succeed? What caused the error? |
| **R2** | **Bot Administrator** | Manages registration, start/stop, configuration, and group behavior of multiple Bot instances | Bot runtime status, group enable/disable policies, configuration consistency |
| **R3** | **Protocol Debugger** | Deep-dives into protocol-layer packets; analyzes protocol behavior, error patterns, and packet structure | Protocol packet content, request-response linkage, error code distribution |
| **R4** | **System Maintainer** | Manages local database health; performs data cleanup, backup, and migration | Database integrity, disk space, audit traceability |

A single real user may act in multiple roles simultaneously.

---

## 5. Business Scenarios

### 5.1 Bot Developer Scenarios

**S-DEV-001: Start a Bot and Observe Message Flow**

A Bot developer starts a registered Bot instance (bound to a QQ account) in UniBot. The system establishes a connection to the protocol endpoint and begins a new debug session. The developer sends a private message to the QQ account bound to the Bot. The system displays the private conversation in the conversation list and shows the message content and sender information in the message area. The developer can view the protocol event corresponding to this message in the message details. The Bot's auto-reply message is also displayed in the conversation.

Related: FR-BOT-001, FR-MSG-001, FR-MSG-002, FR-DBG-001, FR-PKT-001

**S-DEV-002: Trace from a Business Message to Its Protocol Packet**

A developer sees an unexpected reply in a group message. Clicking "View Protocol Packet" on the message, the system displays the inbound event metadata that triggered the Bot's response: protocol type, action name, direction, and timestamp. The developer can further view the raw JSON payload and correlate the Bot's outbound API call and its response.

Related: FR-PKT-001, FR-PKT-002, FR-PKT-003, FR-MSG-005

**S-DEV-003: Filter Failed Protocol Calls**

During debugging, the developer discovers that some Bot messages were not sent successfully. In the protocol packet panel, filtering by `is_error = true`, the system lists all failed protocol calls in reverse chronological order. The developer inspects the failed action names, error messages, and raw request/response JSON. Using this information, the developer identifies an API parameter format issue.

Related: FR-PKT-004, FR-DBG-003

**S-DEV-004: Test Bot Behavior in a Simulated Environment**

The developer starts the built-in virtual server (simulated environment), creates a simulated QQ account and a simulated group. After pointing the Bot to the simulated endpoint, the developer manually sends a group message in the simulated client. The system records the simulated environment's messages, protocol packets, and debug session. The developer can debug the simulated scenario the same way as a real environment, but data is completely isolated from the real environment.

Related: FR-ACC-001, FR-ACC-002, FR-SOC-003, FR-PKT-005

### 5.2 Bot Administrator Scenarios

**S-ADM-001: Create and Configure a New Bot Instance**

A Bot administrator registers a new Bot instance in UniBot, filling in a display name and selecting an existing IM Account to bind. The system generates a unique Bot ID and a default configuration file. The administrator can edit connection settings (protocol type, WebSocket address, auth token) and group behavior settings (mute handling, auto-reply strategy, etc.) in the Bot details.

Related: FR-BOT-001, FR-BOT-002, FR-CFG-001

**S-ADM-002: View Bot Runtime Status and Group List**

The administrator's dashboard displays all Bot instances with runtime status (stopped / running / error). Selecting a running Bot shows its bound groups, including group name, member count, the Bot's group role, and configuration enable state. Groups can be categorized for easier batch management of Bot behavior.

Related: FR-BOT-003, FR-SOC-001, FR-SOC-002, FR-CFG-003

**S-ADM-003: Modify Bot Configuration and Confirm It Takes Effect**

The administrator modifies the reply strategy for a specific group (e.g., disabling auto-reply) and saves the configuration file. The system records a configuration change audit event. When the Bot next receives a message from that group, the new configuration takes effect. The administrator can confirm the change time and content in the audit log.

Related: FR-CFG-002, FR-AUD-001

### 5.3 Protocol Debugger Scenarios

**S-DBG-001: Filter Protocol Packets by Criteria**

A protocol debugger needs to investigate all protocol packets from the last hour. In the protocol packet panel, they select the target Bot, time range, and direction "receive." The system displays all inbound protocol events. The debugger further filters by action name `message.group`, and the system lists only group message events. The debugger exports part of the filtered results for external analysis.

Related: FR-PKT-001, FR-PKT-004, FR-AUD-003

**S-DBG-002: Replay or Export Protocol Packets**

The debugger analyzes a failed API call in depth, viewing the complete request JSON and response JSON (loaded from the local file system). They export the raw JSON file to a specified path or copy the content to the clipboard for replay or analysis in external tools.

Related: FR-PKT-003, FR-AUD-003

**S-DBG-003: Trace Polymorphic Business Associations**

While viewing the group message list, the debugger clicks on a group notification (e.g., a join request). The system navigates from the notification to the protocol event packet that generated it. Conversely, when viewing a protocol packet, the system can display the business objects associated with it (e.g., a specific message, a group request, a group event).

Related: FR-PKT-002, FR-REQ-001

### 5.4 System Maintainer Scenarios

**S-MNT-001: Clean Up Historical Protocol Packet Files**

The system has been running for weeks; protocol packet files occupy significant disk space. The maintainer sets a data retention policy (e.g., keep the last 30 days of protocol packets). The system executes cleanup: first determine the file set from packet records, delete corresponding database records, then attempt to delete the disk JSON files; if file deletion fails, generate a pending-cleanup report. After cleanup, the maintainer verifies that the database still correctly associates messages with retained protocol packets.

Related: FR-AUD-002, DR-CLN-001, DR-CLN-002

**S-MNT-002: Check Database Integrity and Lazy Raw-File Reading**

The maintainer triggers a database integrity check. The system runs SQLite `integrity_check`, validates FK references, and reports orphaned records. Protocol packet raw JSON files are not scanned in bulk; when a user views or exports raw JSON, the system reads `protocol_packets.file_path` and shows "file missing or expired" if the file is missing, unreadable, or invalid JSON.

Related: FR-AUD-004, NFR-REL-002

**S-MNT-003: Backup and Migrate the Database**

The maintainer needs to migrate UniBot to a new computer. The system supports exporting a complete backup package (SQLite file + configuration files + protocol packet files) or exporting only structured data (excluding protocol packet files). On the new computer, the system supports restoring from the backup package.

Related: FR-AUD-005, NFR-MNT-003

---

## 6. Functional Requirements

### 6.1 Account Management (FR-ACC)

**FR-ACC-001: Create and Manage IM Accounts**

| Field | Content |
|-------|---------|
| Description | The system shall support creating and storing basic information for external IM platform accounts, including nickname, avatar, signature, QID, age, sex, and level. Each account shall have an `account_source` attribute marked as `simulated` or `real`. |
| Priority | P0 |
| Roles | R1, R2 |
| Scenarios | S-DEV-004, S-ADM-001 |
| Precondition | None |
| Postcondition | Account information is persisted |
| Acceptance | 1. Create a simulated account with nickname and avatar; it appears in the account list. 2. Sync a real account's profile via the protocol endpoint; the system auto-creates a record with source=real. |
| Data Domain | Identity & Account |

**FR-ACC-002: Simulated/Real Environment Isolation**

| Field | Content |
|-------|---------|
| Description | The system shall ensure that `simulated` and `real` account data are isolated in business logic. Simulated accounts and groups shall only appear in simulated debugging contexts and must not mix with real environment data. |
| Priority | P0 |
| Roles | R1, R3 |
| Scenarios | S-DEV-004 |
| Precondition | Accounts marked as `simulated` and `real` exist |
| Postcondition | Simulated and real data are separated in queries and displays |
| Acceptance | 1. The real environment conversation list does not contain simulated conversations. 2. Attempting to add a simulated account to a real group is blocked. |
| Data Domain | Identity & Account |

**FR-ACC-003: Account Custom Face Management**

| Field | Content |
|-------|---------|
| Description | The system shall support storing custom faces (QQ Marketface) for IM accounts, including face name, emoji package ID, protocol credentials, and local cache path. System built-in faces are loaded from `faces.json` and not stored in the database. |
| Priority | P1 |
| Roles | R1 |
| Scenarios | — |
| Precondition | Target account exists |
| Postcondition | Face metadata is stored; local cache file is downloaded (if remote URL exists) |
| Acceptance | 1. When a message containing a custom face is received, the system auto-records face metadata and associates it with the sender account. 2. Deleting an account cascade-deletes its custom faces. |
| Data Domain | Identity & Account |

### 6.2 Bot Instance Management (FR-BOT)

**FR-BOT-001: Bot Registration and Account Binding**

| Field | Content |
|-------|---------|
| Description | The system shall support registering new Bot instances. Each Bot must have a unique display name and be bound to an existing IM Account. One IM Account may be bound to at most one Bot. Bot configuration is stored in external JSON configuration files; the database stores only the config file path pointer. |
| Priority | P0 |
| Roles | R1, R2 |
| Scenarios | S-ADM-001, S-DEV-001 |
| Precondition | Target IM Account exists |
| Postcondition | Bot is registered; config path is specified |
| Acceptance | 1. Register a Bot and bind an existing account; it appears in the Bot list. 2. Attempting to bind an account already bound to another Bot produces a conflict warning. |
| Data Domain | Identity & Account |

**FR-BOT-002: Bot Runtime Status Management**

| Field | Content |
|-------|---------|
| Description | The system shall support starting and stopping Bot instances. Bot runtime statuses are: `stopped`, `running`, `error`. Status changes shall record audit events. |
| Priority | P0 |
| Roles | R1, R2 |
| Scenarios | S-DEV-001, S-ADM-002 |
| Precondition | Bot is registered with complete configuration |
| Postcondition | Bot status is updated; audit event is recorded |
| Acceptance | 1. After starting a Bot, status updates to `running` within 3 seconds. 2. On abnormal disconnection, status auto-updates to `error`. 3. Every status change has a corresponding audit record. |
| Data Domain | Identity & Account |

**FR-BOT-003: Bot Dashboard**

| Field | Content |
|-------|---------|
| Description | The system shall display an overview of all Bot instances on the dashboard: display name, runtime status, bound account nickname, last start time. (Aggregate statistics such as the number of bound groups are P1 extended scope.) |
| Priority | P0 |
| Roles | R2 |
| Scenarios | S-ADM-002 |
| Precondition | At least one Bot is registered |
| Postcondition | Dashboard reflects database state |
| Acceptance | 1. Dashboard reflects real-time Bot runtime status. 2. Clicking a Bot navigates to its details. |
| Data Domain | Identity & Account |

### 6.3 Conversations & Messages (FR-MSG)

**FR-MSG-001: Message Reception and Persistence**

| Field | Content |
|-------|---------|
| Description | The system shall receive message events from protocol adapters, persist messages to the local database, and uniquely identify each message by scene (private/group/temp) + peer_id + message_seq. Message content shall be stored as a JSON segment array in structured form. The system shall also store relational fields: sender ID, receiver ID (private), group ID (group/temp), quoted message ID (if any). |
| Priority | P0 |
| Roles | R1 |
| Scenarios | S-DEV-001 |
| Precondition | Bot is running; protocol endpoint connection is normal |
| Postcondition | Message is persisted; associated conversation is created or updated |
| Acceptance | 1. After the Bot receives a private message, the message appears in the corresponding conversation with correct message_seq. 2. Duplicate messages with the same scene+peer+seq are ignored (idempotency). |
| Data Domain | Conversations & Messages |

**FR-MSG-002: Conversation List Management**

| Field | Content |
|-------|---------|
| Description | The system shall maintain a conversation list per owning account. Conversation types include `private`, `group`, and `temp`. Each conversation shall maintain a reference to its last message, unread count, last read sequence number, pinned status, and mute status. The conversation list shall be ordered by last message time descending. |
| Priority | P0 |
| Roles | R1, R2 |
| Scenarios | S-DEV-001 |
| Precondition | At least one message exists |
| Postcondition | Conversation list correctly reflects latest messages and unread state |
| Acceptance | 1. After receiving private and group messages for the same account, the conversation list shows both private and group conversations. 2. After receiving 3 messages, unread count = 3. 3. After marking as read, unread count = 0. |
| Data Domain | Conversations & Messages |

**FR-MSG-003: Message Content Rendering**

| Field | Content |
|-------|---------|
| Description | The system shall parse a message's JSON segment array and render it as readable message content. P0 scope covers text segments, @mentions, and basic reply/quote display. P1 scope covers image previews, system faces, custom Marketface, and richer rich-text styling. |
| Priority | P0 (basic rendering); P1 (advanced rich-text rendering) |
| Roles | R1 |
| Scenarios | S-DEV-001 |
| Precondition | Message is stored |
| Postcondition | Message is displayed in readable form |
| Acceptance | 1. (P0) Plain-text messages are rendered correctly. 2. (P0) @mentions are recognizable and highlighted. 3. (P0) Reply/quote shows the quoted message summary. 4. (P1) Image messages show thumbnails or a resource placeholder. 5. (P1) System faces and Marketface show the corresponding placeholder or preview. |
| Data Domain | Conversations & Messages |

**FR-MSG-004: Message Quoting and Recall**

| Field | Content |
|-------|---------|
| Description | The system shall support displaying message quotes (reply) and recording message recalls. When a quoted message is deleted, the quoting side shall display a "[recalled]" placeholder. On recall, the system shall mark `is_recalled` and record the recall time and operator, rather than physically deleting the message record. |
| Priority | P1 |
| Roles | R1 |
| Scenarios | S-DEV-002 |
| Precondition | Quoted or recalled message exists |
| Postcondition | Quote relationship or recall state is recorded |
| Acceptance | 1. Displaying a quoted message correctly renders the quoted message summary. 2. After recall, the message position shows "[recalled]". 3. When a quoted message is recalled, the quoting side shows a placeholder instead of an error. |
| Data Domain | Conversations & Messages |

**FR-MSG-005: Message-to-Protocol-Packet Correlation**

| Field | Content |
|-------|---------|
| Description | The system shall record the protocol event that produced each message (via the denormalized `bot_id` field + the packet's `related_object_type`/`related_object_id`). Users shall be able to navigate from a message detail to the corresponding protocol packet view. |
| Priority | P0 |
| Roles | R1, R3 |
| Scenarios | S-DEV-002 |
| Precondition | The message was produced by a protocol event, and the corresponding protocol packet is recorded |
| Postcondition | The message-protocol packet correlation is traceable from either end |
| Acceptance | 1. Clicking "View Protocol Packet" on a message opens the corresponding inbound protocol event. 2. The `bot_id` field in the message correctly points to the Bot that produced it. |
| Data Domain | Conversations & Messages, Protocol Debugging |

**FR-MSG-006: Message Reactions**

| Field | Content |
|-------|---------|
| Description | The system shall record message reactions (emoji reactions). Addition and removal shall use a toggle model with an `is_add` flag rather than physical deletion. |
| Priority | P2 |
| Roles | R1 |
| Scenarios | — |
| Precondition | Target message exists |
| Postcondition | Reaction record is added or marked as removed |
| Acceptance | 1. After receiving a reaction add event, the message displays the corresponding emoji icon and count. 2. After a reaction remove event, the count decreases. 3. The same user cannot add the same reaction to the same message twice. |
| Data Domain | Conversations & Messages |

**FR-MSG-007: Poke Interactions**

| Field | Content |
|-------|---------|
| Description | The system shall record poke interactions, including sender, target, scene, and timestamp. Poke recall shall be supported. |
| Priority | P2 |
| Roles | R1 |
| Scenarios | — |
| Precondition | Conversation exists |
| Postcondition | Poke interaction is recorded |
| Acceptance | 1. After receiving a poke event, the conversation displays a poke indicator. 2. After a poke is recalled, the recall state is marked. |
| Data Domain | Conversations & Messages |

### 6.4 Friends, Groups & Member Data Mirroring (FR-SOC)

This sub-domain defines mirror-cache requirements for QQ social relationship data. UniBot shall selectively cache friends, groups, group members, and low-frequency data returned by the protocol endpoint as needed for Bot debugging, message display, behavior simulation, and protocol tracing. The system does not guarantee strong consistency with the protocol endpoint, nor does it commit to fully syncing all social data from the QQ client.

**FR-SOC-001: Group Profile Mirroring**

| Field | Content |
|-------|---------|
| Description | The system shall cache group basic information: group name, group avatar, owner, member count, max member count, whole-mute status, and group source (simulated/real). Groups shall support categorization and pinning. |
| Priority | P0 |
| Roles | R1, R2 |
| Scenarios | S-ADM-002 |
| Precondition | Bot is bound to an account that has group chat messages or group profile queries |
| Postcondition | Group information is cached |
| Acceptance | 1. After the Bot receives a group message, the corresponding group auto-appears in the group list. 2. Owner info and member count are displayed correctly. 3. Whole-mute status changes are synchronized and recorded. |
| Data Domain | Social Data |

**FR-SOC-002: Group Member Information Caching**

| Field | Content |
|-------|---------|
| Description | The system shall cache group member basic identity information, and cache extended information when the protocol endpoint provides it. P0 scope covers member account identifier, group card name, and basic role. P1 scope covers special titles, join time, last-sent time, mute status, and other extended fields. |
| Priority | P0 (basic identity); P1 (extended profile) |
| Roles | R1, R2 |
| Scenarios | S-ADM-002 |
| Precondition | Group exists |
| Postcondition | Member information is cached |
| Acceptance | 1. (P0) The group member list displays each member's group card name and basic role (owner/admin/member). 2. (P1) When a member is muted, their mute status updates correctly. 3. (P1) When a member leaves, the member list updates. 4. (P1) Special titles, join time, and last-sent time are cached correctly. |
| Data Domain | Social Data |

**FR-SOC-003: Simulated/Real Group Environment Isolation**

| Field | Content |
|-------|---------|
| Description | Similar to account environment isolation, the system shall ensure `simulated` and `real` groups are isolated in business logic. Group members must only come from the same source environment. |
| Priority | P0 |
| Roles | R1, R3 |
| Scenarios | S-DEV-004 |
| Precondition | Simulated and real groups and accounts exist |
| Postcondition | Cross-environment operations are blocked |
| Acceptance | 1. A simulated group's member list contains only simulated accounts. 2. Attempting to add a real account to a simulated group is blocked. |
| Data Domain | Social Data |

**FR-SOC-004: Friendship Caching**

| Field | Content |
|-------|---------|
| Description | The system shall cache friendships from the owner's perspective (each user sees their own friend list). Each friendship shall support a remark name, category assignment, and pin flag. Friend categories are per-user; each account manages its own categories independently. Every friend must belong to a category (including a default category). |
| Priority | P1 |
| Roles | R1, R2 |
| Scenarios | — |
| Precondition | Target IM Account exists |
| Postcondition | Friendship is cached; category is set |
| Acceptance | 1. Querying an account's friend list shows nickname, remark, and category. 2. A default friend category is auto-created when an account is created. 3. A category with friends cannot be deleted. |
| Data Domain | Social Data |

**FR-SOC-005: Group Category Management**

| Field | Content |
|-------|---------|
| Description | The system shall support per-user group category management. Users can create, delete, and rename group categories, assign groups to categories, and sort by category. Group categories only affect display and organization, not Bot behavior. |
| Priority | P1 |
| Roles | R2 |
| Scenarios | S-ADM-002 |
| Precondition | Target IM Account exists |
| Postcondition | Category is created; groups are assigned |
| Acceptance | 1. After creating a category and assigning groups to it, the category view filters correctly. 2. Deleting a category moves its groups to "uncategorized." |
| Data Domain | Social Data |

**FR-SOC-006: Group Announcement Caching**

| Field | Content |
|-------|---------|
| Description | The system shall cache group announcements, including publisher, content, and image. Announcements shall be viewable by group and time. |
| Priority | P1 |
| Roles | R1, R2 |
| Scenarios | — |
| Precondition | Group exists; protocol endpoint supports announcement queries |
| Postcondition | Announcement content is cached |
| Acceptance | 1. After receiving a group announcement event, the announcement content is cached. 2. The announcement list is displayed in reverse chronological order. |
| Data Domain | Social Data |

**FR-SOC-007: Group File & Folder Caching**

| Field | Content |
|-------|---------|
| Description | The system shall cache group file basic information (name, size, hash, uploader, expiration time, download count) and group folder hierarchy. |
| Priority | P2 |
| Roles | R1, R2 |
| Scenarios | — |
| Precondition | Group exists; protocol endpoint supports file/folder queries |
| Postcondition | File metadata and folder structure are cached |
| Acceptance | 1. The group file list displays file name, size, uploader, and upload time. 2. Nested folder structure is displayed correctly. 3. Expired files are marked as expired. |
| Data Domain | Social Data |

**FR-SOC-008: Group Album & Photo Caching**

| Field | Content |
|-------|---------|
| Description | The system shall cache group album basic information (name, cover) and photo information (URL, description, uploader, size). |
| Priority | P2 |
| Roles | R1, R2 |
| Scenarios | — |
| Precondition | Group exists; protocol endpoint supports album/photo queries |
| Postcondition | Album and photo metadata are cached |
| Acceptance | 1. The group album list displays album names and covers. 2. Photos within an album display thumbnails and descriptions. |
| Data Domain | Social Data |

**FR-SOC-009: Group Essence Message Caching**

| Field | Content |
|-------|---------|
| Description | The system shall cache group essence (pinned) message records, including the referenced message, message sender, and the operator who set the essence. Removing essence deletes the corresponding record. A message may be essenced at most once per group. |
| Priority | P2 |
| Roles | R1 |
| Scenarios | — |
| Precondition | Group exists; target message exists |
| Postcondition | Essence message record is added |
| Acceptance | 1. The essence message list displays the sender and content summary of essenced messages. 2. The same message cannot be essenced twice in the same group. 3. Removing essence deletes the record. |
| Data Domain | Social Data |

### 6.5 Request & Event Handling (FR-REQ)

**FR-REQ-001: Friend Request Management**

| Field | Content |
|-------|---------|
| Description | The system shall record friend requests, including initiator, target, comment, and processing state. The state machine is: pending → accepted / rejected / ignored. Duplicate pending requests between the same user pair shall not be created. |
| Priority | P1 |
| Roles | R1, R2 |
| Scenarios | S-DBG-003 |
| Precondition | Both initiator and target accounts exist |
| Postcondition | Request is stored with state = pending |
| Acceptance | 1. After receiving a friend request, it appears in the request list with state = pending. 2. After accepting, state becomes accepted; the system auto-creates a bidirectional friendship. 3. When a pending request already exists between the same pair, duplicates are ignored. |
| Data Domain | Social Data |

**FR-REQ-002: Group Notification/Request Management**

| Field | Content |
|-------|---------|
| Description | The system shall record group notifications that require handling (e.g., join requests, invitation notifications), including initiator, target, notification type, and state. The state machine is the same as friend requests. |
| Priority | P1 |
| Roles | R1, R2 |
| Scenarios | S-DBG-003 |
| Precondition | Group exists; initiator account exists |
| Postcondition | Group notification is stored |
| Acceptance | 1. After receiving a join request, a notification appears in the group details/notifications. 2. After accepting a join request, the member is auto-added to the group member list. |
| Data Domain | Social Data |

**FR-REQ-003: Group Event Recording**

| Field | Content |
|-------|---------|
| Description | The system shall record events occurring within groups (e.g., member join, member mute, essence set). Event records are append-only (no modification, no deletion). Event payloads are stored as JSON. |
| Priority | P1 |
| Roles | R1, R3 |
| Scenarios | — |
| Precondition | Group exists |
| Postcondition | Event is appended |
| Acceptance | 1. The group event list is displayed in reverse chronological order and can be filtered by event type. 2. Event records contain the complete raw payload JSON. |
| Data Domain | Social Data |

### 6.6 Protocol Packet Tracing (FR-PKT)

**FR-PKT-001: Complete Protocol Packet Recording**

| Field | Content |
|-------|---------|
| Description | The system shall record every protocol interaction. Each record shall contain: protocol type (Milky/OneBot-v11/OneBot-v12), direction (send/receive), action name, associated Bot, associated debug session, error flag (is_error), timestamp, and raw JSON file path (`file_path`). The raw packet JSON shall be stored in the local file system; the database shall store only the file path and index metadata. |
| Priority | P0 |
| Roles | R1, R3 |
| Scenarios | S-DEV-001, S-DEV-002, S-DBG-001 |
| Precondition | Bot is running; protocol endpoint communication is normal |
| Postcondition | Packet index is stored; raw JSON is written to disk; file_path is stored on the packet record |
| Acceptance | 1. After the Bot sends an API call, the packet list shows a record with direction = send. 2. After receiving a protocol event, a record with direction = receive appears. 3. The packet JSON file exists at the specified path and is readable. |
| Data Domain | Protocol Debugging |

**FR-PKT-002: Protocol Packet-to-Business-Object Association**

| Field | Content |
|-------|---------|
| Description | The system shall support associating protocol packets with the business objects they produce (via `related_object_type` + `related_object_id` polymorphic association). Association types include: message, group_request, group_event, etc. Conversely, from a business object (e.g., a message) the system shall allow reverse tracing to the corresponding protocol packet. |
| Priority | P0 |
| Roles | R1, R3 |
| Scenarios | S-DEV-002, S-DBG-003 |
| Precondition | Both the protocol packet and associated business object are stored |
| Postcondition | Bidirectional association is queryable |
| Acceptance | 1. Clicking from a message navigates to the corresponding protocol packet detail. 2. From a protocol packet, the associated business object (e.g., message summary, group event summary) can be viewed. |
| Data Domain | Protocol Debugging, Conversations & Messages, Social Data |

**FR-PKT-003: Raw Protocol Packet Viewing & Export**

| Field | Content |
|-------|---------|
| Description | The system shall support loading and displaying the complete raw JSON of a protocol packet from the file system. Copying to clipboard and exporting to a specified path shall be supported. |
| Priority | P1 |
| Roles | R3 |
| Scenarios | S-DEV-002, S-DBG-002 |
| Precondition | The packet's raw JSON file exists on disk and is readable |
| Postcondition | Packet content is displayed in the UI or exported |
| Acceptance | 1. Clicking a protocol packet record displays the raw JSON (with syntax highlighting). 2. The copy button copies the complete JSON to clipboard. 3. The export button saves the JSON file to a user-specified path. |
| Data Domain | Protocol Debugging |

**FR-PKT-004: Multi-Dimension Protocol Packet Filtering**

| Field | Content |
|-------|---------|
| Description | The system shall support filtering the protocol packet list by the following dimensions in combination: Bot, debug session, protocol type, direction (send/receive), action name (fuzzy match), error status (is_error), time range, related object type. Filtered results shall be ordered by time descending. |
| Priority | P0 |
| Roles | R1, R3 |
| Scenarios | S-DEV-003, S-DBG-001 |
| Precondition | Protocol packet records exist in the database |
| Postcondition | Filtered results contain only records matching all criteria |
| Acceptance | 1. After selecting a Bot + time range, only that Bot's packets in that range are shown. 2. Filtering by is_error=true shows only failed packets. 3. Combined Bot + action + is_error filtering produces the correct intersection. |
| Data Domain | Protocol Debugging |

**FR-PKT-005: Simulated Endpoint Protocol Packet Recording**

| Field | Content |
|-------|---------|
| Description | The system shall also record protocol interactions from the simulated endpoint, distinguishable from real-source packets. |
| Priority | P1 |
| Roles | R1, R3 |
| Scenarios | S-DEV-004 |
| Precondition | Simulated endpoint is started and generates protocol interactions |
| Postcondition | Simulated packets are distinguishable from real packets |
| Acceptance | 1. The simulated endpoint's packet list is separated from or filterable by source. |
| Data Domain | Protocol Debugging |

**FR-PKT-006: Protocol Packet File Path & Lazy Check**

| Field | Content |
|-------|---------|
| Description | The system shall store the raw JSON `file_path` directly on the protocol packet record. When viewing or exporting raw JSON, the system shall read that path directly; if the file is missing, unreadable, or invalid JSON, it shall show a "file missing or expired" message instead of maintaining extra file metadata or background verification state. |
| Priority | P0 |
| Roles | R4 |
| Scenarios | S-MNT-001, S-MNT-002 |
| Precondition | Protocol packet record and corresponding JSON file exist |
| Postcondition | Under normal writes, the packet record's file_path points to a readable disk JSON file |
| Acceptance | 1. Under normal writes, the packet record and disk JSON file are consistent. 2. After manually deleting the JSON file, clicking to view raw JSON shows a file-missing message. 3. If database write fails, residual temp files are discoverable by cleanup or directory maintenance. |
| Data Domain | Protocol Debugging |

### 6.7 Debug Session Management (FR-DBG)

**FR-DBG-001: Automatic Debug Session Creation & Lifecycle**

| Field | Content |
|-------|---------|
| Description | The system shall automatically create a new debug session when a Bot starts, recording the session name, description (optional), and start time. When the Bot stops or exits abnormally, the system shall record the session end time. Debug sessions are owned by a Bot. |
| Priority | P0 |
| Roles | R1, R3 |
| Scenarios | S-DEV-001 |
| Precondition | Bot starts |
| Postcondition | Debug session is created; start time is recorded |
| Acceptance | 1. When a Bot starts, a new debug session is auto-created. 2. When a Bot stops, the current session's end time is recorded. 3. A Bot can have multiple rounds of debug sessions. |
| Data Domain | Protocol Debugging |

**FR-DBG-002: Message and Packet Aggregation Within a Debug Session**

| Field | Content |
|-------|---------|
| Description | The system shall mark messages and protocol packets with the debug session ID (via the `session_id` field) in which they were produced. When a user selects a debug session, the system shall display all messages and protocol packets produced during that session. |
| Priority | P0 |
| Roles | R1, R3 |
| Scenarios | S-DEV-001, S-DEV-003 |
| Precondition | Debug session is created; Bot is running |
| Postcondition | Messages and packets have their session_id populated |
| Acceptance | 1. In the debug session view, only messages from that session are shown. 2. In the debug session view, only protocol packets from that session are shown. 3. Comparing results from two different sessions yields isolated analysis. |
| Data Domain | Protocol Debugging, Conversations & Messages |

**FR-DBG-003: Debug Session List & Review**

| Field | Content |
|-------|---------|
| Description | The system shall display the history list of all debug sessions per Bot, filterable by time range. Selecting a historical debug session shall display its messages and protocol packets in read-only mode. |
| Priority | P1 |
| Roles | R1, R3 |
| Scenarios | S-DEV-003 |
| Precondition | At least one debug session is completed |
| Postcondition | Historical session content is viewable |
| Acceptance | 1. The debug session list is ordered by start time descending. 2. Clicking a historical session displays its messages and packets. 3. Messages and packets in historical sessions are read-only. |
| Data Domain | Protocol Debugging |

### 6.8 Configuration Management (FR-CFG)

**FR-CFG-001: Bot Configuration File Management**

| Field | Content |
|-------|---------|
| Description | The system shall support reading and writing Bot configuration files. The system shall validate the basic structure (JSON validity) of configuration files. After a user modifies a configuration, the system shall indicate whether a Bot restart is required to apply the new configuration. |
| Priority | P0 |
| Roles | R1, R2 |
| Scenarios | S-ADM-001 |
| Precondition | Bot is registered; configuration file path is known |
| Postcondition | Configuration file is updated |
| Acceptance | 1. After editing and saving Bot config, the file content is updated. 2. Invalid JSON format prompts the user. 3. Modifying config while Bot is running prompts that restart is needed. |
| Data Domain | System Governance |

**FR-CFG-002: Configuration Change Recording**

| Field | Content |
|-------|---------|
| Description | The system shall record an audit event on configuration changes, including change time, content, and operator. |
| Priority | P1 |
| Roles | R2, R4 |
| Scenarios | S-ADM-003 |
| Precondition | A configuration change occurred |
| Postcondition | Audit event is recorded |
| Acceptance | 1. Every configuration modification is visible in the audit log. 2. The audit record contains a summary of before/after configuration content. |
| Data Domain | System Governance |

**FR-CFG-003: Bot Group Behavior Configuration**

| Field | Content |
|-------|---------|
| Description | The system shall support configuring Bot behavior policies at the group granularity level, including but not limited to: auto-reply toggle, reply mode, per-group allowlist/blocklist state. Configuration content is stored in the Bot's JSON configuration file, not in separate tables. |
| Priority | P1 |
| Roles | R2 |
| Scenarios | S-ADM-002, S-ADM-003 |
| Precondition | Bot is registered; target group exists |
| Postcondition | Group-level configuration is updated |
| Acceptance | 1. After disabling auto-reply for a specific group, the Bot no longer replies to messages in that group. 2. Configuration changes are verifiable at the file level. |
| Data Domain | System Governance |

### 6.9 Audit, Export & System Maintenance (FR-AUD)

**FR-AUD-001: Operational Audit Log**

| Field | Content |
|-------|---------|
| Description | The system shall record audit events for all critical operations, including: Bot start/stop, message deletion, configuration changes, data cleanup, etc. Each audit event shall contain: event type, actor, target type, target ID, and structured detail JSON. Audit events are append-only within their retention period (no modification, no deletion by normal business flows); expired events may be archived or cleaned up per policy. |
| Priority | P1 |
| Roles | R4 |
| Scenarios | S-ADM-003 |
| Precondition | A critical operation occurred |
| Postcondition | Audit event is persisted |
| Acceptance | 1. Bot start and stop auto-record audit events. 2. Manually deleting a message records an audit event. 3. The audit event list can be filtered by event type and time. |
| Data Domain | System Governance |

**FR-AUD-002: Scheduled Protocol Packet & File Cleanup**

| Field | Content |
|-------|---------|
| Description | The system shall support automatic cleanup of historical protocol packets by time range: first determine the file set from packet records, delete corresponding database records, then attempt to delete the disk JSON files; if file deletion fails, record a pending-cleanup report. The cleanup operation itself shall record an audit event. |
| Priority | P1 |
| Roles | R4 |
| Scenarios | S-MNT-001 |
| Precondition | Protocol packets exist beyond the retention period |
| Postcondition | Packets and files beyond the retention period are cleaned up |
| Acceptance | 1. Set retention to 30 days; after cleanup, packets older than 30 days are deleted, along with their files. 2. Cleanup does not affect message records (messages remain). 3. The cleanup operation is recorded in the audit log. |
| Data Domain | Protocol Debugging, System Governance |

**FR-AUD-003: Data Export**

| Field | Content |
|-------|---------|
| Description | The system shall support exporting: a filtered protocol packet list (with metadata, optionally including raw JSON files), and message history for a specified conversation. Export format shall be structured JSON or readable text. Before export, the system shall prompt the user to confirm the export scope. |
| Priority | P1 |
| Roles | R3, R4 |
| Scenarios | S-DBG-001, S-DBG-002 |
| Precondition | Filter criteria are set; export scope is confirmed |
| Postcondition | Data is exported to a user-specified path |
| Acceptance | 1. Export 100 filtered protocol packet metadata records; the file contains all fields. 2. Export message history containing message content JSON and sender information. |
| Data Domain | Protocol Debugging, Conversations & Messages |

**FR-AUD-004: Data Integrity Check**

| Field | Content |
|-------|---------|
| Description | The system shall support triggering a database integrity check. Checks include: whether FK references are valid (no dangling foreign keys), and whether the database file itself is corrupted (via SQLite `integrity_check`). Raw protocol files are not bulk-checked through extra file metadata; missing files are reported lazily when users view or export raw JSON. |
| Priority | P2 |
| Roles | R4 |
| Scenarios | S-MNT-002 |
| Precondition | Database and file system are accessible |
| Postcondition | Integrity report is generated |
| Acceptance | 1. After running integrity check, no FK references point to deleted parent records. 2. SQLite integrity_check passes. 3. After deleting one packet JSON file, viewing that packet's raw JSON shows a file-missing message. |
| Data Domain | System Governance, Protocol Debugging |

**FR-AUD-005: Database Backup & Restore**

| Field | Content |
|-------|---------|
| Description | The system shall support exporting a complete backup package (SQLite file + configuration files + protocol packet files). Restore from a backup package shall be supported. Backup and restore operations shall record audit events. |
| Priority | P2 |
| Roles | R4 |
| Scenarios | S-MNT-003 |
| Precondition | Database is functioning normally |
| Postcondition | Backup package is created or restore is completed |
| Acceptance | 1. After creating a backup package, restoring on a new environment allows normal startup and data viewing. 2. The backup file contains the database, configuration, and registered protocol packet files. |
| Data Domain | System Governance |

**FR-AUD-006: Application Settings Management**

| Field | Content |
|-------|---------|
| Description | The system shall support Key-Value storage for global application settings. Key settings include: database schema version (for migration tracking), data retention policy parameters (e.g., protocol packet retention days), and UI preferences. |
| Priority | P0 |
| Roles | R4 |
| Scenarios | S-MNT-001 |
| Precondition | System first launch or settings change |
| Postcondition | Settings are persisted |
| Acceptance | 1. The system can persistently record the current database schema version and read it at startup or migration. 2. After changing the retention days, the next cleanup follows the new policy. 3. Setting value types are constrained to string/int/bool/json. |
| Data Domain | System Governance |

---

## 7. Data Requirements

### 7.1 Data Domain Overview

The system's data requirements are organized into **5 data domains**. Each domain defines the data categories the system must persist and their business meaning. The "Data Domain" field in each functional requirement indicates which domains are involved. Specific entity mappings and table structures are provided in the database design document.

| Data Domain | Meaning | Related Functional Areas | Core Entity Concepts |
|-------------|---------|--------------------------|----------------------|
| **Identity & Account** | External IM account identity, Bot instance registration, and binding relationships | FR-ACC, FR-BOT | IM Accounts, Bot Instances, Account Faces, Friend Categories, Group Categories |
| **Conversations & Messages** | Messages, conversation containers, and message-level interactions (quotes, reactions, pokes) | FR-MSG | Conversations, Messages, Reactions, Pokes |
| **Social Data** | Friendships, groups, group members, group content assets, social requests, and events | FR-SOC, FR-REQ | Friends, Groups, Group Members, Announcements, Files, Albums, Essence Messages, Friend Requests, Group Requests, Group Events |
| **Protocol Debugging** | Protocol packet indices, raw payload files, debug session aggregation | FR-PKT, FR-DBG | Protocol Packets, Packet File Registrations, Debug Sessions |
| **System Governance** | Application settings, audit logs, data cleanup and backup policies | FR-CFG, FR-AUD | Application Settings, Audit Events |

### 7.2 Core Business Data

Core business data is the data on which the system's primary flow (Bot run → message send/receive → conversation management) directly depends:

- **IM Accounts**: The identity foundation; all Bot bindings, message ownership, and friendships are account-based
- **Bot Instances**: The debugging targets; each must bind one IM Account
- **Conversations**: Message organization containers; uniquely identified by owner account + scene + peer
- **Messages**: Core assets; uniquely identified by scene + peer_id + message_seq; content stored as JSON segment arrays

This data must guarantee ACID transaction consistency.

### 7.3 Protocol Debugging Data

Protocol debugging data is UniBot's key differentiator from a generic IM client:

- **Protocol Packets**: Records of every protocol interaction, storing structured index fields (protocol type, direction, action, error flag); raw JSON payloads stored in the file system
- **Raw File Path**: Each protocol packet stores `file_path` directly; the file is read lazily when viewed or exported
- **Debug Sessions**: Abstractions of Bot run cycles, aggregating messages and packets

Protocol debugging data supports polymorphic association to business objects (via `related_object_type` + `related_object_id`) but does not enforce FK constraints (the referenced target is determined at runtime).

### 7.4 Cache & Mirror Data

Cache and mirror data originates from external protocol endpoints. The system passively receives and locally caches this data to support display and debugging context. The system does not commit to strong-consistency synchronization with protocol endpoints:

- **Friendships, Groups, Group Members**: Basic profile data for display and group management
- **Group Announcements, Files/Folders, Albums/Photos, Essence Messages**: Low-frequency mirror data, all at P1/P2 priority
- **Account Faces**: Face rendering cache

Principle: store whatever the protocol endpoint returns; do not proactively pull entities the endpoint does not support querying.

### 7.5 System Configuration & Audit Data

- **Application Settings**: Global Key-Value configuration, including current schema version, data retention policy, and UI preferences. The specific migration record structure shall be determined by the database design document.
- **Audit Events**: Immutable records of all critical operations
- **Bot Configuration Files**: JSON files stored in the file system; database stores only the config path pointer

### 7.6 Data Lifecycle

| Data Category | Creation Trigger | Update Frequency | Deletion Policy | Cleanup Method |
|---------------|------------------|------------------|-----------------|----------------|
| Account Info | First sync or manual creation | Low (on profile change) | Manual delete; by default, releases strong references to historical messages; user may optionally fully purge associated data | — |
| Bot Instances | Manual registration | Low (on config change) | Manual delete; runtime state, config pointers, and debug sessions handled per policy | — |
| Messages | On protocol message event arrival | High (message frequency) | No auto-cleanup by default; supports manual deletion by user or full purge per account | — |
| Conversations | On first message arrival | High (every new message updates) | Manual delete; by default, only removes conversation view state; whether to delete messages is determined by user action | — |
| Social Data | Protocol event-driven sync | Medium (on profile/member change) | Manual delete of group/friend; deleted on full account purge | — |
| Protocol Packets | Every protocol interaction while Bot is running | Very high | Periodic cleanup by retention policy | Scheduled task by TTL |
| Packet Files | Synchronously on packet record creation | Very low (write once) | Follows packet cleanup flow; record pending-cleanup status on deletion failure | Background task retry |
| Debug Sessions | Auto-create on Bot start | Low (end time update) | Manual delete; may retain summary or clean up related markers per user choice | — |
| Audit Events | On critical operation occurrence | Medium | Append-only within retention period; no modification or deletion by normal flows | Archived or cleaned up per retention policy |

### 7.7 Data Retention & Cleanup Policies

| ID | Content |
|----|---------|
| DR-CLN-001 | The system shall support configuring the retention period for protocol packets and packet files (default 30 days). Data beyond the retention period shall be automatically cleaned up by a scheduled task. |
| DR-CLN-002 | When cleaning up protocol packets, the system shall first determine the set of files to delete, then delete database records and attempt disk file deletion; if disk deletion fails, the system shall record a cleanup-pending state or generate a cleanup report for background task retry. |
| DR-CLN-003 | Message records are not auto-cleaned up by default; manual deletion by the user is required. |
| DR-CLN-004 | General operational audit events default to 90-day retention; security-critical audit events default to permanent retention. General events beyond the retention period are auto-cleaned up by a scheduled task. |
| DR-CLN-005 | Cache and mirror data (friends, groups, members, etc.) is not auto-cleaned up; it is CASCADE-deleted when the associated account is deleted. |

---

## 8. Non-Functional Requirements

### 8.1 Performance (NFR-PERF)

**Test environment**: CPU 4+ cores, 8 GB RAM, SSD storage, SQLite WAL mode enabled, indexed queries used. Measurement methodology: 10 consecutive runs, P95 value.

| ID | Content |
|----|---------|
| NFR-PERF-001 | With 1 million messages in the local SQLite database, loading the most recent 30 messages for a conversation shall take ≤ 300ms. |
| NFR-PERF-002 | With 500,000 protocol packets, filtering the most recent 100 by Bot + time range (last 7 days) shall take ≤ 500ms. |
| NFR-PERF-003 | Loading the conversation list (most recent 50 conversations) shall take ≤ 300ms. |
| NFR-PERF-004 | The UI shall maintain ≥ 30 FPS while messages and protocol packets are being continuously written. |
| NFR-PERF-005 | Cold application startup (including database connection) shall take ≤ 2 seconds. |

### 8.2 Capacity & Scalability (NFR-CAP)

| ID | Content |
|----|---------|
| NFR-CAP-001 | The system shall support registering at least 20 Bot instances. |
| NFR-CAP-002 | The system shall store at least 1 million messages in a single SQLite database file without significant query degradation. |
| NFR-CAP-003 | The system shall store at least 500,000 protocol packet records in a single SQLite database file. |
| NFR-CAP-004 | Protocol packet files shall be stored in shards by date, Bot ID, or packet_id prefix; the system shall support at least 500,000 protocol packet JSON files (1–500 KB each) in total. |
| NFR-CAP-005 | The system shall support caching at least 500 groups and 100,000 group member records. |

### 8.3 Reliability & Recovery (NFR-REL)

| ID | Content |
|----|---------|
| NFR-REL-001 | On abnormal exit (process crash, power loss), the SQLite database file shall not be corrupted (relies on SQLite WAL mode and atomic commits). |
| NFR-REL-002 | When a database record references a packet file path but the disk file is missing, the system shall detect this when the user views or exports raw JSON and show an anomaly message rather than crashing. |
| NFR-REL-003 | When a configuration file has invalid JSON format, the system shall prevent Bot startup and display a clear error message, rather than using corrupted configuration. |
| NFR-REL-004 | If an exception occurs during packet write, the database shall not leave protocol packet records pointing to JSON files that were not successfully written. |

### 8.4 Security & Privacy (NFR-SEC)

| ID | Content |
|----|---------|
| NFR-SEC-001 | Database fields, configuration files, and log files generated by UniBot itself shall not actively write plaintext login credentials, Cookies, or authentication tokens. |
| NFR-SEC-002 | Raw protocol packets returned by the protocol endpoint may contain sensitive fields (e.g., tokens, cookies). UniBot does not guarantee automatic sanitization; the system shall prompt the user to inspect content and confirm the risk before exporting, copying, or sharing protocol packets. |
| NFR-SEC-003 | Before exporting protocol packet data, the system shall prompt the user to confirm the export scope and warn that it may contain private information. |
| NFR-SEC-004 | The system shall support encrypting the SQLite database file (via SQLite encryption extension or Tauri file encryption), with the encryption key set by the user. |
| NFR-SEC-005 | Local log files shall not output message content, raw protocol packet JSON, or any text fields that may contain user chat content. |
| NFR-SEC-006 | When deleting an IM Account, the system shall release strong references from the account to historical messages (sender SET NULL); historical messages shall be retained as debugging assets by default. The user may optionally perform a full purge of all data associated with the account. |

### 8.5 Maintainability (NFR-MNT)

| ID | Content |
|----|---------|
| NFR-MNT-001 | The system shall persistently record the database schema version. Each schema change shall have a corresponding migration script and migration verification record. |
| NFR-MNT-002 | The system's data access layer and protocol adapter layer shall have clear interface boundaries. Replacing the protocol endpoint or database engine shall not affect the business logic layer. |
| NFR-MNT-003 | The system shall provide a database migration function, supporting seamless upgrade from an older schema version. |

### 8.6 Observability (NFR-OBS)

| ID | Content |
|----|---------|
| NFR-OBS-001 | The system shall output structured logs (JSON format) with configurable log levels (debug/info/warn/error). |
| NFR-OBS-002 | Log content shall include critical operational events: Bot start/stop, protocol connect/disconnect, message send/receive counts, errors, and exceptions. |
| NFR-OBS-003 | The `is_error` field on all protocol interaction records shall accurately reflect the error state returned by the protocol endpoint, serving as a core observability metric. |
| NFR-OBS-004 | Audit events shall cover all critical operations; the event type enumeration shall be exhaustive. |

### 8.7 Compatibility & Portability (NFR-CMP)

| ID | Content |
|----|---------|
| NFR-CMP-001 | The system shall be able to simultaneously connect to Milky, OneBot-v11, and OneBot-v12 protocol endpoints. Protocol differences shall be normalized by the Adapter layer. |
| NFR-CMP-002 | When protocol endpoint fields are missing or do not match the expected schema, the system shall apply a "best-effort preservation" principle: retain the raw data, mark missing fields, and not reject storage due to incomplete fields. |
| NFR-CMP-003 | Database files and protocol packet files shall be migratable between Windows, macOS, and Linux, unaffected by OS file path separator differences (use relative paths or a unified path format). |
| NFR-CMP-004 | SQLite database files shall be readable and writable on SQLite 3.35.0+, ensuring compatibility with the SQLite version bundled with the Tauri SQL plugin. |

---

## 9. Requirements Priority Matrix

### 9.1 P0 — Core (system primary flow cannot function without these)

| ID | Name | Sub-Domain |
|----|------|------------|
| FR-ACC-001 | Create and Manage IM Accounts | Account Management |
| FR-ACC-002 | Simulated/Real Environment Isolation | Account Management |
| FR-BOT-001 | Bot Registration and Account Binding | Bot Instance Management |
| FR-BOT-002 | Bot Runtime Status Management | Bot Instance Management |
| FR-BOT-003 | Bot Dashboard | Bot Instance Management |
| FR-MSG-001 | Message Reception and Persistence | Conversations & Messages |
| FR-MSG-002 | Conversation List Management | Conversations & Messages |
| FR-MSG-003a | Basic Message Rendering | Conversations & Messages |
| FR-MSG-005 | Message-to-Protocol-Packet Correlation | Conversations & Messages |
| FR-SOC-001 | Group Profile Mirroring | Friends, Groups & Members |
| FR-SOC-002a | Basic Group Member Identity | Friends, Groups & Members |
| FR-SOC-003 | Simulated/Real Group Environment Isolation | Friends, Groups & Members |
| FR-PKT-001 | Complete Protocol Packet Recording | Protocol Packet Tracing |
| FR-PKT-002 | Protocol Packet-to-Business-Object Association | Protocol Packet Tracing |
| FR-PKT-004 | Multi-Dimension Protocol Packet Filtering | Protocol Packet Tracing |
| FR-PKT-006 | File Path & Lazy Read Check | Protocol Packet Tracing |
| FR-DBG-001 | Automatic Debug Session Creation & Lifecycle | Debug Session Management |
| FR-DBG-002 | Message and Packet Aggregation Within a Debug Session | Debug Session Management |
| FR-CFG-001 | Bot Configuration File Management | Configuration Management |
| FR-AUD-006 | Application Settings Management | Audit, Export & Maintenance |

### 9.2 P1 — Important (core experience and completeness)

| ID | Name | Sub-Domain |
|----|------|------------|
| FR-ACC-003 | Account Custom Face Management | Account Management |
| FR-MSG-003b | Advanced Rich-Text Rendering | Conversations & Messages |
| FR-MSG-004 | Message Quoting and Recall | Conversations & Messages |
| FR-SOC-002b | Extended Group Member Info | Friends, Groups & Members |
| FR-SOC-004 | Friendship Caching | Friends, Groups & Members |
| FR-SOC-005 | Group Category Management | Friends, Groups & Members |
| FR-SOC-006 | Group Announcement Caching | Friends, Groups & Members |
| FR-REQ-001 | Friend Request Management | Request & Event Handling |
| FR-REQ-002 | Group Notification/Request Management | Request & Event Handling |
| FR-REQ-003 | Group Event Recording | Request & Event Handling |
| FR-PKT-003 | Raw Protocol Packet Viewing & Export | Protocol Packet Tracing |
| FR-PKT-005 | Simulated Endpoint Protocol Packet Recording | Protocol Packet Tracing |
| FR-DBG-003 | Debug Session List & Review | Debug Session Management |
| FR-CFG-002 | Configuration Change Recording | Configuration Management |
| FR-CFG-003 | Bot Group Behavior Configuration | Configuration Management |
| FR-AUD-001 | Operational Audit Log | Audit, Export & Maintenance |
| FR-AUD-002 | Scheduled Protocol Packet & File Cleanup | Audit, Export & Maintenance |
| FR-AUD-003 | Data Export | Audit, Export & Maintenance |

### 9.3 P2 — Optional (nice-to-have or low-frequency)

| ID | Name | Sub-Domain |
|----|------|------------|
| FR-MSG-006 | Message Reactions | Conversations & Messages |
| FR-MSG-007 | Poke Interactions | Conversations & Messages |
| FR-SOC-007 | Group File & Folder Caching | Friends, Groups & Members |
| FR-SOC-008 | Group Album & Photo Caching | Friends, Groups & Members |
| FR-SOC-009 | Group Essence Message Caching | Friends, Groups & Members |
| FR-AUD-004 | Data Integrity Check | Audit, Export & Maintenance |
| FR-AUD-005 | Database Backup & Restore | Audit, Export & Maintenance |

---

## 10. Out of Scope

The following items define the hard boundaries of the UniBot system:

| ID | Content |
|----|---------|
| OUT-001 | The system does not guarantee complete synchronization of a QQ client's full message history; it only saves messages received through the Bot or actively pulled during UniBot's runtime. |
| OUT-002 | The system is not a long-term chat archive tool; historical protocol packets are automatically cleaned up according to the retention policy. |
| OUT-003 | The system does not guarantee strong-consistency mirroring for low-frequency assets such as group files, albums, and announcements; these are passively cached from whatever the protocol endpoint returns. |
| OUT-004 | The system does not directly manage or store QQ login credentials (passwords, tickets, tokens, etc.); these are managed by the external protocol endpoint. |
| OUT-005 | The system does not guarantee field-level consistency across protocol endpoints (Milky/OneBot-v11/OneBot-v12); protocol differences are best-effort normalized by the Adapter layer. |
| OUT-006 | The system does not provide multi-user collaboration; a single UniBot instance is designed for single-user local use. |
| OUT-007 | The system is not a QQ Bot hosting platform: it does not provide cloud deployment for Bots, does not manage automatic Bot lifecycle recovery, and does not provide Bot runtime monitoring alerts. |
| OUT-008 | The system does not automatically execute Bot behavior: message replies, group management, and friend approval are all performed by the Bot itself (via the external protocol endpoint); UniBot only records and displays. |

---

## 11. Verification & Validation Strategy

### 11.1 P0 Verification Principles

All P0 requirements shall satisfy the following verification principles:

1. **Independently verifiable**: Does not depend on other incomplete P0 requirements
2. **Executable acceptance criteria**: Acceptance criteria are stated as observable, measurable outcomes
3. **Boundary coverage**: At minimum, covers the normal path and at least one abnormal/boundary path
4. **Data consistency**: Requirements involving data writes must include verification that "database records match UI display"

### 11.2 Scenario-Level Acceptance Tests

| Scenario | Acceptance Test Description |
|----------|----------------------------|
| S-DEV-001 | **End-to-end**: Register a Bot, bind a real account, start the Bot, send a private message to the bound account: (a) the conversation list shows the private conversation; (b) the message content is displayed correctly; (c) a debug session is created; (d) the protocol packet list contains the corresponding inbound event |
| S-DEV-002 | **Correlation traceability**: Select a group message, click "View Protocol Packet" — the system navigates to the corresponding packet detail, showing protocol type, action name, and raw JSON |
| S-DEV-003 | **Error filtering**: Create 3 Bots, 2 debug sessions, a mix of normal and failed packets; filter by is_error=true — results contain only failed packets |
| S-DEV-004 | **Environment isolation**: Create a simulated account and group, start the simulated endpoint, send a group message: (a) the real environment's conversation list does not contain the simulated group conversation; (b) the simulated group's members do not include real accounts |
| S-ADM-001 | **Bot creation**: Register a new Bot, fill in display name, select binding account, save: (a) the Bot list shows the new Bot; (b) clicking shows details; (c) a configuration file is created |
| S-ADM-003 | **Configuration change**: Modify Bot group behavior config (disable auto-reply), save: (a) the config file content is updated; (b) an audit event is recorded; (c) after Bot restart, the new config takes effect |
| S-DBG-001 | **Multi-dimension filtering**: Select a specific Bot + last 24 hours + direction receive + action contains "message" — results show only matching items |
| S-MNT-001 | **Data cleanup**: Set retention to 7 days; the system has packets from 10 days ago; execute cleanup: (a) 10-day-old packet records are deleted; (b) corresponding disk JSON files are deleted; (c) packets within 7 days remain intact |
| S-MNT-002 | **Lazy read check**: Manually delete one packet JSON file; click "View Raw JSON" for that packet: (a) the system does not crash; (b) the UI shows that the file is missing or expired |

### 11.3 Data Consistency Verification

| ID | Verification Content |
|----|----------------------|
| V-DC-001 | Packet write consistency: Under a stress test of writing 1000 protocol packets, the database packet record count = disk JSON file count, and each record's file_path locates its corresponding file |
| V-DC-002 | Message idempotency: Repeatedly send a message event with the same scene+peer+seq 100 times; only 1 message record exists in the database |
| V-DC-003 | Account deletion consistency: After deleting an IM Account, historical message sender references are set to NULL with message content retained; Bot bindings, friendships, conversation states, and cache data are handled per the deletion policy; when the user chooses "full purge," the account's associated messages, conversations, cache, and protocol correlation data are cleaned or sanitized |
| V-DC-004 | Unread count: After receiving N consecutive messages in the same conversation, unread_count = N; after marking as read, unread_count = 0 |

### 11.4 Non-Functional Verification

| ID | Verification Content |
|----|----------------------|
| V-NFR-001 | Performance: In a 1M-message database, load the most recent 30 messages for a conversation; use `performance.now()` timing; 10 consecutive runs; P95 value ≤ 300ms |
| V-NFR-002 | Reliability: Force-terminate the process during a message write; after restart, verify database is uncorrupted via `PRAGMA integrity_check` |
| V-NFR-003 | Security: Use a sensitive-field dictionary (token, cookie, ticket, authorization, session, credential, access_token, refresh_token, etc.) to scan database columns, configuration files, application logs, and export files; confirm that UniBot itself does not actively write plaintext credentials. For raw protocol packet files, verify only the export risk warning and user confirmation flow. |
| V-NFR-004 | Compatibility: Connect Milky and OneBot-v11 endpoints separately; send a group message with the same content; both messages display basic content (text, sender) correctly in UniBot; protocol-specific fields are tagged with their source |

---

## Appendix A: Requirements Traceability Matrix

| Scenario | Functional Requirements | Data Domains | Acceptance Test |
|----------|------------------------|--------------|-----------------|
| S-DEV-001 Start Bot & observe messages | FR-BOT-001, FR-BOT-002, FR-MSG-001, FR-MSG-002, FR-MSG-003, FR-DBG-001, FR-DBG-002, FR-PKT-001 | Identity & Account, Conversations & Messages, Protocol Debugging | S-DEV-001 end-to-end |
| S-DEV-002 Trace message to packet | FR-PKT-001, FR-PKT-002, FR-PKT-003, FR-MSG-004, FR-MSG-005 | Conversations & Messages, Protocol Debugging | S-DEV-002 correlation trace |
| S-DEV-003 Filter failed calls | FR-PKT-004, FR-DBG-003 | Protocol Debugging | S-DEV-003 error filter |
| S-DEV-004 Simulated env testing | FR-ACC-001, FR-ACC-002, FR-SOC-003, FR-PKT-005 | Identity & Account, Social Data, Protocol Debugging | S-DEV-004 env isolation |
| S-ADM-001 Create & configure Bot | FR-BOT-001, FR-BOT-002, FR-CFG-001 | Identity & Account, System Governance | S-ADM-001 Bot creation |
| S-ADM-002 View status & groups | FR-BOT-003, FR-SOC-001, FR-SOC-002, FR-CFG-003 | Identity & Account, Social Data | — |
| S-ADM-003 Modify config & verify | FR-CFG-001, FR-CFG-002, FR-AUD-001 | System Governance | S-ADM-003 config change |
| S-DBG-001 Filter packets | FR-PKT-001, FR-PKT-004 | Protocol Debugging | S-DBG-001 multi-filter |
| S-DBG-002 Export packets | FR-PKT-003, FR-AUD-003 | Protocol Debugging | — |
| S-DBG-003 Trace polymorphic assoc | FR-PKT-002, FR-REQ-001, FR-REQ-002 | Protocol Debugging, Social Data | — |
| S-MNT-001 Clean up packets | FR-AUD-002, FR-AUD-006, FR-PKT-006 | Protocol Debugging, System Governance | S-MNT-001 data cleanup |
| S-MNT-002 Database integrity and lazy read check | FR-AUD-004, FR-PKT-006 | System Governance, Protocol Debugging | S-MNT-002 lazy read check |
| S-MNT-003 Backup & migrate | FR-AUD-005 | System Governance | — |
| — (Data mirror: friends) | FR-SOC-004, FR-SOC-005 | Social Data | Individual FR acceptance |
| — (Data mirror: group content) | FR-SOC-006, FR-SOC-007, FR-SOC-008, FR-SOC-009 | Social Data | Individual FR acceptance |
| — (Message interactions) | FR-MSG-006, FR-MSG-007 | Conversations & Messages | Individual FR acceptance |
| — (Faces & events) | FR-ACC-003, FR-REQ-002, FR-REQ-003 | Identity & Account, Social Data | Individual FR acceptance |

## Appendix B: References

| Reference | Description |
|-----------|-------------|
| Milky Documentation (https://milky.ntqqrev.org/) | QQ bot application interface standard; message API and event model reference |
| OneBot v11 Specification (https://github.com/botuniverse/onebot-11) | Universal chat bot interface standard v11 |
| OneBot v12 Specification (https://12.onebot.dev/) | Standardized event structures and APIs |
| LangBot (https://github.com/langbot-app/LangBot) | Production-grade multi-platform IM Bot platform; reference for Bot management and plugin architecture |
| SQLite Documentation (https://www.sqlite.org/docs.html) | Embedded database; reference for WAL mode, partial indexes, CHECK constraints |
| Tauri Documentation (https://v2.tauri.app/) | Cross-platform desktop application framework; UniBot's runtime foundation |
| ISO/IEC/IEEE 29148:2018 | Systems and software engineering — Requirements engineering international standard; this document's structure references this standard |
| EARS (Easy Approach to Requirements Syntax) | Requirements authoring methodology; this document's functional requirement descriptions reference this method |

---

*Document Version: v1.0 | Last Updated: 2026-05-15 | Language: English*
