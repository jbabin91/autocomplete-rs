# Implementation Tasks

## 1. Daemon Foundation

- [ ] 1.1 Implement Unix socket server with tokio
- [ ] 1.2 Handle socket cleanup on shutdown
- [ ] 1.3 Accept concurrent connections
- [ ] 1.4 Implement connection pooling/lifecycle management
- [ ] 1.5 Add graceful shutdown handling

## 2. Communication Protocol

- [ ] 2.1 Define JSON message format for requests (buffer, cursor position)
- [ ] 2.2 Define JSON message format for responses (suggestions array)
- [ ] 2.3 Implement request deserializer
- [ ] 2.4 Implement response serializer
- [ ] 2.5 Add protocol version field for future compatibility
- [ ] 2.6 Handle malformed requests gracefully

## 3. TUI Implementation

- [ ] 3.1 Create CompletionUI struct with Ratatui
- [ ] 3.2 Implement dropdown rendering with List widget
- [ ] 3.3 Add keyboard navigation (Up/Down arrows)
- [ ] 3.4 Implement selection with Enter key
- [ ] 3.5 Add cancellation with Esc key
- [ ] 3.6 Style selected item with highlight color
- [ ] 3.7 Calculate correct screen position for dropdown
- [ ] 3.8 Handle screen boundaries (don't render off-screen)

## 4. ZLE Integration

- [ ] 4.1 Create zsh widget function
- [ ] 4.2 Capture BUFFER and CURSOR from zle
- [ ] 4.3 Send request to daemon via Unix socket
- [ ] 4.4 Parse response and trigger TUI
- [ ] 4.5 Insert selected completion into buffer
- [ ] 4.6 Bind widget to key (Alt+Space default)
- [ ] 4.7 Auto-start daemon if not running

## 5. Daemon Lifecycle

- [ ] 5.1 Check if daemon is already running (PID file or socket check)
- [ ] 5.2 Start daemon in background
- [ ] 5.3 Write PID file for tracking
- [ ] 5.4 Implement daemon stop command
- [ ] 5.5 Handle daemon crash recovery

## 6. Error Handling

- [ ] 6.1 Handle socket connection failures
- [ ] 6.2 Handle daemon not running
- [ ] 6.3 Handle timeout on daemon response
- [ ] 6.4 Display user-friendly error messages
- [ ] 6.5 Log errors for debugging

## 7. Testing

- [ ] 7.1 Unit tests for message serialization/deserialization
- [ ] 7.2 Integration test for daemon request/response cycle
- [ ] 7.3 Manual testing with zsh widget
- [ ] 7.4 Test TUI rendering and navigation
- [ ] 7.5 Test daemon cleanup on exit
