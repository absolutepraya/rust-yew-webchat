## Yew Chat

### Original Code

![Original Code](./images/1.png)

### Creative Features: Reply and Timestamp

#### Timestamp Feature

![Timestamp](./images/2.png)

Each message now displays the time it was sent, making it easier to follow conversations and understand message chronology. Times are shown in HH:MM:SS format.

**Implementation Details:**

- Message timestamps are received from the server as Unix timestamps (milliseconds since epoch)
- The `chrono` crate handles conversion from Unix time to formatted time strings
- Timestamps appear in the top-right corner of each message bubble
- The timestamp is visually subtle (gray, smaller text) to avoid distracting from message content

This feature adds important temporal context to conversations, especially useful in active group chats or when referencing past messages.

#### Reply Feature

![Reply](./images/3.png)
![Reply](./images/4.png)
![Reply](./images/5.png)

A reply system has been implemented that allows users to respond directly to specific messages:

**User Flow:**

1. User clicks the reply icon (💬) button on any message
2. A blue "Replying to [username]" indicator appears above the message input
3. The indicator includes a preview of the original message text
4. User types their response and clicks send
5. The new message appears with the quoted original message above it

**Technical Implementation:**

- Messages now include an optional `reply_to` field with the original message's ID, sender, and content
- The server has been modified to handle and broadcast this reply metadata
- Reply relationships are preserved even as new messages arrive
- The reply UI is styled with a distinctive blue accent bar and indented layout
- Users can cancel a reply by clicking the "x" button on the reply indicator

**Benefits:**

- Creates clear conversation threads in busy group chats
- Maintains context when responding to earlier messages
- Helps new participants understand conversation flow
- Reduces confusion when multiple topics are discussed simultaneously

This reply system significantly improves the chat experience by adding structure to conversations without sacrificing the simplicity of the interface.

### Bonus: Rust WebSocket Server Implementation

#### Why Convert from JavaScript to Rust?

The original WebSocket server for this chat application was built with JavaScript. As a bonus task, I've reimplemented the server using Rust to replace the JavaScript version while maintaining full compatibility with the existing Yew chat client.

#### Implementation Details

The Rust WebSocket server implements identical functionality to the JavaScript version:

1. **WebSocket Communication Protocol**:

   - Maintains the same JSON message format used by the JavaScript server
   - Handles user registration, messaging, and user listing operations
   - Supports the reply feature we added to the chat application
   - Uses the same port (8080) to ensure compatibility with the client

2. **Technical Implementation**:

   - Built using Tokio and tokio-tungstenite for asynchronous WebSocket handling
   - Uses Serde for JSON serialization/deserialization
   - Implements connection management with a thread-safe user map
   - Provides the same real-time broadcasting capabilities
   - Includes connection health checks every 5 seconds

3. **Key Improvements**:
   - Type safety through Rust's strong type system
   - Increased performance through Rust's zero-cost abstractions
   - Better error handling with Rust's Result type
   - Improved concurrency with Tokio's asynchronous runtime
   - Enhanced maintainability with clearer codebase structure

#### How to Run the Rust WebSocket Server

```bash
cd RustWebsocketServer
cargo run
```

The server will listen on 127.0.0.1:8080 just like the JavaScript version, so the client application requires no changes to connect to it.

#### Comparison: JavaScript vs. Rust Server

**JavaScript Server Advantages**:

- Simpler initial setup with fewer lines of code
- More familiar syntax for web developers
- Easier to modify for those with web development background
- Doesn't require compilation, enabling faster iteration

**Rust Server Advantages**:

- Stronger type safety reduces runtime errors
- Superior performance, especially for high-load scenarios
- Better memory safety without garbage collection pauses
- Enhanced concurrency handling with async/await
- Comprehensive error handling at compile-time

**Personal Preference**:
I prefer the Rust implementation for several reasons:

1. The type system prevents many common errors found in JavaScript
2. Performance scales better for larger applications
3. Error handling is more explicit and thorough
4. Concurrency is more safely managed
5. The code is more maintainable in the long term

While the JavaScript version is easier to quickly prototype with, the Rust version provides a more robust foundation for a production-ready chat application. The initial investment in Rust's steeper learning curve pays off with a more reliable and efficient server.
