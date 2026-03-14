//! Backend services.

pub mod agent_manager;
pub mod ascii_art;
pub mod session_store;
pub mod theme;
pub mod worktree_manager;

pub use agent_manager::AgentManager;
pub use session_store::SessionStore;
pub use theme::Theme;
