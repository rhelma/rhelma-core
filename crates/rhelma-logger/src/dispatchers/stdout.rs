use std::io::Write;

use crate::event::LogEvent;
use crate::extensions::LogDispatcher;

#[derive(Debug, Clone, Default)]
pub struct StdoutDispatcher;

impl LogDispatcher for StdoutDispatcher {
    fn dispatch(&self, event: LogEvent) {
        match serde_json::to_string(&event) {
            Ok(line) => println!("{line}"),
            Err(e) => {
                // Do not print the full event (may contain PII); emit a minimal message.
                eprintln!("[rhelma-logger] StdoutDispatcher: failed to serialize LogEvent: {e}");
            }
        }
    }

    /// 🔥 NEW: برای flush_interval و shutdown
    fn flush(&self) {
        if let Err(e) = std::io::stdout().flush() {
            // Avoid printing full events; this is a minimal internal error.
            crate::state::report_internal_error(&format!(
                "StdoutDispatcher: stdout flush failed: {e}"
            ));
        }
    }

    fn box_clone(&self) -> Box<dyn LogDispatcher + Send + Sync> {
        Box::new(self.clone())
    }
}
