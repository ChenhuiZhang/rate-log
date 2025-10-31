//! # Rate Log
//!
//! A Rust library for rate-limited logging that prevents spam by tracking message frequency
//! and duration. This crate helps reduce log noise by detecting repeated messages and only
//! outputting warnings when configurable limits are exceeded.
//!
//! ## Features
//!
//! - **Count-based rate limiting**: Limit by number of repeated message occurrences
//! - **Duration-based rate limiting**: Limit by accumulated time between repeated messages
//! - **Unified tracking**: Always tracks both count and duration for comprehensive reporting
//! - **Smart duration formatting**: Automatically formats durations in appropriate units (ms, s, m, h)
//! - **Message deduplication**: Automatically resets counters when different messages are logged
//! - **Zero-cost abstractions**: Minimal runtime overhead with compile-time optimizations
//! - **Test-friendly**: Built-in output capture for unit testing
//!
//! ## Quick Start
//!
//! ```rust
//! use rate_log::{RateLog, Limit};
//! use std::time::Duration;
//!
//! // Create a rate limiter that allows up to 5 repeated messages
//! let mut rate_log = RateLog::new(Limit::Rate(5));
//!
//! // First occurrence of any message is always printed immediately
//! rate_log.log("This is a new message");  // Prints: "This is a new message"
//!
//! // Log the same message multiple times - no output until limit exceeded
//! for i in 0..7 {
//!     rate_log.log("This is a new message");
//! }
//! // After 5 repetitions, it will output: "Message: \"This is a new message\" repeat for 5 times in the past 10ms"
//!
//! // Different message gets printed immediately and resets counter
//! rate_log.log("Different message");  // Prints: "Different message"
//! ```
//!
//! ## Rate Limiting Types
//!
//! ### Count-based Limiting (`Limit::Rate`)
//!
//! Tracks the number of times the same message is logged consecutively:
//!
//! ```rust
//! use rate_log::{RateLog, Limit};
//!
//! let mut logger = RateLog::new(Limit::Rate(3));
//!
//! logger.log("Error occurred");     // 1st occurrence - printed immediately: "Error occurred"
//! logger.log("Error occurred");     // 2nd occurrence - counted silently
//! logger.log("Error occurred");     // 3rd occurrence - counted silently
//! logger.log("Error occurred");     // 4th occurrence - triggers: "Message: \"Error occurred\" repeat for 3 times in the past 15ms"
//! logger.log("Different error");    // New message - printed immediately: "Different error"
//! ```
//!
//! ### Duration-based Limiting (`Limit::Duration`)
//!
//! Accumulates the time elapsed between consecutive calls with the same message:
//!
//! ```rust
//! use rate_log::{RateLog, Limit};
//! use std::time::Duration;
//! use std::thread;
//!
//! let mut logger = RateLog::new(Limit::Duration(Duration::from_secs(1)));
//!
//! logger.log("Periodic event");      // 1st occurrence - printed immediately: "Periodic event"
//! thread::sleep(Duration::from_millis(300));
//! logger.log("Periodic event");      // 300ms accumulated - silent
//! thread::sleep(Duration::from_millis(800));
//! logger.log("Periodic event");      // 1100ms total - triggers: "Message: \"Periodic event\" repeat for 2 times in the past 1s"
//! ```
//!
//! ## Behavior
//!
//! - **New message printing**: Every new/different message is immediately printed to stdout
//! - **Unified tracking**: Always tracks both message count and elapsed duration regardless of limit type
//! - **Silent repetitions**: Repeated messages are counted silently until limit exceeded
//! - **Smart duration formatting**: Automatically displays duration in appropriate units (ms, s, m, h) with whole numbers
//! - **Comprehensive warnings**: Rate limit violations show both count and duration: "Message: \"text\" repeat for X times in the past Yms"
//! - **Counter reset**: Switching to a different message resets all counters and prints the new message
//!
//! ## Use Cases
//!
//! - **Error logging**: Prevent log spam from repeated error conditions
//! - **Debug output**: Control verbose debug message frequency
//! - **Performance monitoring**: Rate-limit performance warnings
//! - **Network logging**: Manage connection retry message frequency
//! - **System monitoring**: Control repeated system state notifications

use std::time::{Duration, Instant};

/// Formats a duration into a human-readable string with whole numbers only.
/// Automatically chooses the most appropriate unit (hours, minutes, seconds, or milliseconds).
fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    if total_secs >= 3600 {
        format!("{}h", total_secs / 3600)
    } else if total_secs >= 60 {
        format!("{}m", total_secs / 60)
    } else if total_secs >= 1 {
        format!("{}s", total_secs)
    } else {
        format!("{}ms", duration.as_millis())
    }
}

/// Defines the type and threshold for rate limiting.
///
/// `Limit` specifies how rate limiting should be applied - either by counting
/// message occurrences or by measuring time duration between repeated messages.
///
/// # Examples
///
/// ```rust
/// use rate_log::Limit;
/// use std::time::Duration;
///
/// // Allow up to 10 repeated messages before triggering rate limit
/// let count_limit = Limit::Rate(10);
///
/// // Allow up to 5 seconds of accumulated time between repeated messages
/// let time_limit = Limit::Duration(Duration::from_secs(5));
/// ```
#[derive(Debug, PartialEq, PartialOrd)]
pub enum Limit {
    /// Count-based rate limiting.
    ///
    /// Triggers when the same message is repeated more than the specified number of times.
    /// The counter resets when a different message is logged.
    ///
    /// # Example
    /// ```rust
    /// use rate_log::{RateLog, Limit};
    ///
    /// let mut logger = RateLog::new(Limit::Rate(3));
    /// // Will trigger rate limit warning after 4th identical message
    /// ```
    Rate(u32),

    /// Duration-based rate limiting.
    ///
    /// Triggers when the accumulated time between consecutive identical messages
    /// exceeds the specified duration. Time is measured between actual calls,
    /// providing real-world timing behavior.
    ///
    /// # Example
    /// ```rust
    /// use rate_log::{RateLog, Limit};
    /// use std::time::Duration;
    ///
    /// let mut logger = RateLog::new(Limit::Duration(Duration::from_millis(500)));
    /// // Will trigger if total elapsed time between identical messages > 500ms
    /// ```
    Duration(Duration),
}

#[derive(Debug)]
struct State {
    count: u32,
    duration: Duration,
    last_timestamp: Option<Instant>,
}

impl State {
    fn new() -> Self {
        State {
            count: 0,
            duration: Duration::from_secs(0),
            last_timestamp: None,
        }
    }

    fn reset(&mut self) {
        self.count = 0;
        self.duration = Duration::from_secs(0);
        self.last_timestamp = None;
    }

    fn exceeds_limit(&self, limit: &Limit) -> bool {
        match limit {
            Limit::Rate(limit_count) => self.count >= *limit_count,
            Limit::Duration(limit_duration) => self.duration >= *limit_duration,
        }
    }
}

/// A rate limiting logger that tracks message frequency and duration.
///
/// `RateLog` monitors how frequently the same message is logged and can enforce
/// limits based on either count (number of occurrences) or time duration.
/// It will output the message first time and then until the limits are exceeded.
pub struct RateLog {
    /// The maximum allowed limit for rate limiting.
    /// This defines the threshold that triggers rate limit exceeded warnings.
    /// For `Rate(n)`: maximum number of repeated messages allowed
    /// For `Duration(d)`: maximum time duration allowed for repeated messages
    limit: Limit,

    /// The current tracking state containing count, duration, and timestamp.
    /// Always tracks both message count and elapsed duration regardless of limit type,
    /// enabling comprehensive rate limit reporting.
    current: State,

    /// The last message that was logged.
    /// Used to detect when a different message is being logged, which resets
    /// the rate limiting counters. Only identical messages contribute to rate limiting.
    message: String,

    /// Test-only field that captures output messages for verification in unit tests.
    /// This field is only present when compiled with test configuration and allows
    /// tests to verify the exact output without relying on stdout capture.
    #[cfg(test)]
    output: String,
}

impl RateLog {
    /// Creates a new `RateLog` instance with the specified limit.
    ///
    /// The rate limiter starts with clean state - no previous messages tracked
    /// and all counters at zero.
    ///
    /// # Arguments
    ///
    /// * `limit` - The rate limiting threshold to enforce
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rate_log::{RateLog, Limit};
    /// use std::time::Duration;
    ///
    /// // Create count-based rate limiter
    /// let count_limiter = RateLog::new(Limit::Rate(5));
    ///
    /// // Create duration-based rate limiter
    /// let time_limiter = RateLog::new(Limit::Duration(Duration::from_secs(2)));
    /// ```
    pub fn new(limit: Limit) -> Self {
        let current = State::new();

        RateLog {
            limit,
            current,
            message: String::new(),
            #[cfg(test)]
            output: String::new(),
        }
    }

    /// Logs a message with rate limiting applied.
    ///
    /// This method immediately prints any new or different message to stdout, then tracks
    /// repeated messages and enforces the configured rate limit. Repeated messages are
    /// counted silently until the limit is exceeded.
    ///
    /// # Output Behavior
    ///
    /// - **New/different message**: Immediately printed to stdout and resets all counters
    /// - **Repeated message**: Counted silently (no immediate output)
    /// - **Limit exceeded**: Prints rate limit warning to stdout
    ///
    /// # Rate Limiting Behavior
    ///
    /// - **Count-based**: Increments counter for each repeated message
    /// - **Duration-based**: Accumulates elapsed time between repeated messages
    /// - **Message change**: Resets all tracking state and prints the new message
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to log and track for rate limiting
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rate_log::{RateLog, Limit};
    ///
    /// let mut logger = RateLog::new(Limit::Rate(2));
    ///
    /// logger.log("Starting up");          // Prints: "Starting up"
    /// logger.log("Error occurred");       // Prints: "Error occurred" (different message)
    /// logger.log("Error occurred");       // Silent (1st repetition)
    /// logger.log("Error occurred");       // Silent (2nd repetition)
    /// logger.log("Error occurred");       // Prints: "Message: \"Error occurred\" repeat for 2 times in the past 15ms"
    /// logger.log("Shutting down");        // Prints: "Shutting down" (different message)
    /// ```
    pub fn log(&mut self, msg: &str) {
        let now = Instant::now();

        if self.message != msg {
            self.message = msg.to_string();
            self.current.reset();

            println!("{msg}");

            #[cfg(test)]
            {
                self.output.push_str(msg);
            }
        } else {
            self.current.count += 1;

            if let Some(last_call) = self.current.last_timestamp {
                let elapsed = now.duration_since(last_call);
                self.current.duration += elapsed;
            }

            if self.current.exceeds_limit(&self.limit) {
                let output = format!(
                    "Message: \"{}\" repeat for {} times in the past {}",
                    msg,
                    self.current.count,
                    format_duration(self.current.duration)
                );
                println!("{output}");

                self.current.reset();

                println!("{output}");

                #[cfg(test)]
                {
                    self.output.push_str(&output);
                }
            }
        }

        self.current.last_timestamp = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_log_exceed_time() {
        let mut rate_log = RateLog::new(Limit::Rate(3));

        // First call - should not exceed
        rate_log.log("message1");
        assert_eq!(rate_log.output, "message1");
        rate_log.output.clear();

        // Second call - should not exceed (current becomes 1, limit is 3)
        rate_log.log("message1");
        assert_eq!(rate_log.output, "");

        // Third call - should not exceed (current becomes 2, limit is 3)
        rate_log.log("message1");
        assert_eq!(rate_log.output, "");

        // Fourth call - should exceed (current becomes 3, limit is 3)
        rate_log.log("message1");
        assert_eq!(
            rate_log.output,
            "Message: \"message1\" repeat for 3 times in the past 0ms"
        );
        rate_log.output.clear();

        // Fifth call - should not exceed (current becomes 1, limit is 3)
        rate_log.log("message1");
        assert_eq!(rate_log.output, "");

        // Sixth call - should not exceed (current becomes 2, limit is 3)
        rate_log.log("message1");
        assert_eq!(rate_log.output, "");

        // Seventh call - should exceed (current becomes 3, limit is 3)
        rate_log.log("message1");
        assert_eq!(
            rate_log.output,
            "Message: \"message1\" repeat for 3 times in the past 0ms"
        );
        rate_log.output.clear();
    }

    #[test]
    fn test_rate_log_exceed_duration() {
        use std::thread;

        let mut rate_log = RateLog::new(Limit::Duration(Duration::from_millis(50)));

        // First call
        rate_log.log("message2");
        assert_eq!(rate_log.output, "message2");
        rate_log.output.clear();

        // Second call after short delay - should not exceed
        thread::sleep(Duration::from_millis(20));
        rate_log.log("message2");
        assert_eq!(rate_log.output, "");

        // Third call after longer delay - should exceed the 50ms limit
        thread::sleep(Duration::from_millis(40));
        rate_log.log("message2");
        assert_eq!(
            rate_log.output,
            "Message: \"message2\" repeat for 2 times in the past 60ms"
        );
        rate_log.output.clear();

        rate_log.log("message2");
        assert_eq!(rate_log.output, "");

        thread::sleep(Duration::from_millis(50));
        rate_log.log("message2");
        assert_eq!(
            rate_log.output,
            "Message: \"message2\" repeat for 2 times in the past 50ms"
        );
        rate_log.output.clear();
    }
}
