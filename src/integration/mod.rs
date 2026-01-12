//! External tool integrations
//!
//! Connects Rembrandt with Beads (tasks), Porque (decisions), and Agent Mail.

pub mod agent_mail;
pub mod beads;
pub mod porque;

/// Trait for external tool integrations
pub trait Integration {
    /// Check if the integration is available
    fn is_available(&self) -> bool;

    /// Get the name of the integration
    fn name(&self) -> &'static str;
}
