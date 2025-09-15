//! Progress reporting for import/export operations

/// Trait for receiving progress updates during import/export operations
pub trait ProgressHandler {
    /// Called to report progress
    ///
    /// # Parameters
    /// - `percentage`: Progress as a value between 0.0 and 1.0
    /// - `message`: Optional descriptive message about the current operation
    ///
    /// # Returns
    /// Return `true` to continue the operation, `false` to cancel
    fn update(&mut self, percentage: f32, message: Option<&str>) -> bool;
}

/// A simple progress handler that prints to stdout
pub struct PrintProgressHandler {
    last_percentage: i32,
}

impl PrintProgressHandler {
    /// Create a new print progress handler
    pub fn new() -> Self {
        Self {
            last_percentage: -1,
        }
    }
}

impl Default for PrintProgressHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressHandler for PrintProgressHandler {
    fn update(&mut self, percentage: f32, message: Option<&str>) -> bool {
        let current_percentage = (percentage * 100.0) as i32;

        // Only print when percentage changes significantly
        if current_percentage != self.last_percentage {
            if let Some(msg) = message {
                println!("Progress: {}% - {}", current_percentage, msg);
            } else {
                println!("Progress: {}%", current_percentage);
            }
            self.last_percentage = current_percentage;
        }

        true // Continue operation
    }
}

/// A progress handler that stores progress information without printing
pub struct SilentProgressHandler {
    percentage: f32,
    message: Option<String>,
    cancelled: bool,
}

impl SilentProgressHandler {
    /// Create a new silent progress handler
    pub fn new() -> Self {
        Self {
            percentage: 0.0,
            message: None,
            cancelled: false,
        }
    }

    /// Get the current progress percentage
    pub fn percentage(&self) -> f32 {
        self.percentage
    }

    /// Get the current progress message
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Check if the operation was cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Cancel the operation
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
}

impl Default for SilentProgressHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressHandler for SilentProgressHandler {
    fn update(&mut self, percentage: f32, message: Option<&str>) -> bool {
        self.percentage = percentage;
        self.message = message.map(|s| s.to_string());
        !self.cancelled
    }
}

/// A progress handler that calls a closure
pub struct ClosureProgressHandler<F>
where
    F: FnMut(f32, Option<&str>) -> bool,
{
    closure: F,
}

impl<F> ClosureProgressHandler<F>
where
    F: FnMut(f32, Option<&str>) -> bool,
{
    /// Create a new closure-based progress handler
    pub fn new(closure: F) -> Self {
        Self { closure }
    }
}

impl<F> ProgressHandler for ClosureProgressHandler<F>
where
    F: FnMut(f32, Option<&str>) -> bool,
{
    fn update(&mut self, percentage: f32, message: Option<&str>) -> bool {
        (self.closure)(percentage, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silent_progress_handler() {
        let mut handler = SilentProgressHandler::new();

        assert_eq!(handler.percentage(), 0.0);
        assert!(handler.message().is_none());
        assert!(!handler.is_cancelled());

        let result = handler.update(0.5, Some("Testing"));
        assert!(result);
        assert_eq!(handler.percentage(), 0.5);
        assert_eq!(handler.message(), Some("Testing"));

        handler.cancel();
        assert!(handler.is_cancelled());

        let result = handler.update(0.8, None);
        assert!(!result);
    }

    #[test]
    fn test_closure_progress_handler() {
        let mut call_count = 0;
        let mut last_percentage = 0.0;

        {
            let mut handler = ClosureProgressHandler::new(|percentage, _message| {
                call_count += 1;
                last_percentage = percentage;
                true
            });

            handler.update(0.3, Some("Test"));
            handler.update(0.7, None);
        }

        assert_eq!(call_count, 2);
        assert_eq!(last_percentage, 0.7);
    }
}
