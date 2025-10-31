# Rate Log

[![Crates.io](https://img.shields.io/crates/v/rate-log.svg)](https://crates.io/crates/rate-log)
[![Documentation](https://docs.rs/rate-log/badge.svg)](https://docs.rs/rate-log)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](#license)

A Rust library for rate-limited logging that prevents spam by tracking message frequency and duration. This crate helps reduce log noise by detecting repeated messages and only outputting warnings when configurable limits are exceeded.

*Inspired by the [log_hz](https://crates.io/crates/log_hz) crate.*

## Features

- **Count-based rate limiting**: Limit by number of repeated message occurrences
- **Duration-based rate limiting**: Limit by accumulated time between repeated messages
- **Unified tracking**: Always tracks both count and duration for comprehensive reporting
- **Smart duration formatting**: Automatically formats durations in appropriate units (ms, s, m, h)
- **Message deduplication**: Automatically resets counters when different messages are logged
- **Zero-cost abstractions**: Minimal runtime overhead with compile-time optimizations
- **Test-friendly**: Built-in output capture for unit testing

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
rate-log = "0.1.0"
```

### Basic Usage

```rust
use rate_log::{RateLog, Limit};
use std::time::Duration;

// Create a rate limiter that allows up to 5 repeated messages
let mut rate_log = RateLog::new(Limit::Rate(5));

// First occurrence of any message is always printed immediately
rate_log.log("This is a new message");  // Prints: "This is a new message"

// Log the same message multiple times - no output until limit exceeded
for i in 0..7 {
    rate_log.log("This is a new message");
}
// After 5 repetitions, it will output:
// "Message: \"This is a new message\" repeat for 5 times in the past 10ms"

// Different message gets printed immediately and resets counter
rate_log.log("Different message");  // Prints: "Different message"
```

## Rate Limiting Types

### Count-based Limiting (`Limit::Rate`)

Tracks the number of times the same message is logged consecutively:

```rust
use rate_log::{RateLog, Limit};

let mut logger = RateLog::new(Limit::Rate(3));

logger.log("Error occurred");     // 1st occurrence - printed immediately: "Error occurred"
logger.log("Error occurred");     // 2nd occurrence - counted silently
logger.log("Error occurred");     // 3rd occurrence - counted silently
logger.log("Error occurred");     // 4th occurrence - triggers warning:
                                  // "Message: \"Error occurred\" repeat for 3 times in the past 15ms"
logger.log("Different error");    // New message - printed immediately: "Different error"
```

### Duration-based Limiting (`Limit::Duration`)

Accumulates the time elapsed between consecutive calls with the same message:

```rust
use rate_log::{RateLog, Limit};
use std::time::Duration;
use std::thread;

let mut logger = RateLog::new(Limit::Duration(Duration::from_secs(1)));

logger.log("Periodic event");      // 1st occurrence - printed immediately: "Periodic event"
thread::sleep(Duration::from_millis(300));
logger.log("Periodic event");      // 300ms accumulated - silent
thread::sleep(Duration::from_millis(800));
logger.log("Periodic event");      // 1100ms total - triggers warning:
                                   // "Message: \"Periodic event\" repeat for 2 times in the past 1s"
```

## Use Cases

### Error Logging
Prevent log spam from repeated error conditions:

```rust
use rate_log::{RateLog, Limit};

let mut error_logger = RateLog::new(Limit::Rate(10));

// This will only show the first occurrence and then a summary after 10 repetitions
for _ in 0..50 {
    error_logger.log("Database connection failed");
}
```

### Performance Monitoring
Rate-limit performance warnings:

```rust
use rate_log::{RateLog, Limit};
use std::time::Duration;

let mut perf_logger = RateLog::new(Limit::Duration(Duration::from_secs(30)));

// Only warn about slow responses every 30 seconds of accumulated time
if response_time > threshold {
    perf_logger.log("Slow response detected");
}
```

### Network Logging
Manage connection retry message frequency:

```rust
use rate_log::{RateLog, Limit};

let mut net_logger = RateLog::new(Limit::Rate(5));

// Limit connection retry spam
while !connected {
    net_logger.log("Retrying connection...");
    // attempt connection
}
```

## Behavior

- **New message printing**: Every new/different message is immediately printed to stdout
- **Unified tracking**: Always tracks both message count and elapsed duration regardless of limit type
- **Silent repetitions**: Repeated messages are counted silently until limit exceeded
- **Smart duration formatting**: Automatically displays duration in appropriate units (ms, s, m, h) with whole numbers
- **Comprehensive warnings**: Rate limit violations show both count and duration: "Message: \"text\" repeat for X times in the past Yms"
- **Counter reset**: Switching to a different message resets all counters and prints the new message

## API Documentation

### `RateLog::new(limit: Limit) -> Self`

Creates a new rate limiter with the specified threshold.

### `RateLog::log(&mut self, msg: &str)`

Logs a message with rate limiting applied. New messages are printed immediately, repeated messages are tracked until limits are exceeded.

### `Limit::Rate(u32)`

Count-based rate limiting. Triggers when the same message exceeds the specified count.

### `Limit::Duration(Duration)`

Duration-based rate limiting. Triggers when accumulated time between repeated messages exceeds the specified duration.

## Testing

Run the test suite:

```bash
cargo test
```

Run with output to see the rate limiting in action:

```bash
cargo test -- --nocapture
```

## Todo

- [ ] Add configurable output formatting
- [ ] Support for custom output writers (not just stdout)
- [ ] Add reset methods for manual counter clearing
- [ ] Benchmarks and performance optimization
- [ ] `no_std` support for embedded environments
- [ ] Integration with popular logging frameworks (`log`, `tracing`)
- [ ] Configurable timestamp precision
- [ ] Memory usage optimization for long-running applications
- [ ] Async support for non-blocking rate limiting
- [ ] Multi-threaded safety with `Arc<Mutex<RateLog>>`
- [ ] Persistent rate limiting across application restarts
- [ ] Rate limiting policies (exponential backoff, sliding window)
- [ ] Metrics integration (Prometheus, StatsD)

## Development

```bash
# Clone the repository
git clone https://github.com/your-username/rate-log.git
cd rate-log

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Check documentation
cargo doc --open

# Run clippy for linting
cargo clippy

# Format code
cargo fmt
```

## License

This project is licensed under the MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT).

## Contribution

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to:
- Update tests as appropriate
- Follow the existing code style
- Run `cargo fmt` and `cargo clippy` before submitting
- Add documentation for new features
