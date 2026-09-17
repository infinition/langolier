pub mod assistant;
pub mod bundle;
#[cfg(feature = "desktop")]
mod cmd_assistants;
#[cfg(feature = "desktop")]
mod cmd_chat;
#[cfg(feature = "desktop")]
mod cmd_sources;
#[cfg(feature = "desktop")]
mod cmd_watches;
pub mod db;
#[cfg(feature = "desktop")]
mod desktop;
pub mod engine;
pub mod ingest;
pub mod llm;
mod office;
pub mod pipeline;
pub mod rag;
pub mod server;
pub mod telegram;
pub mod watch;
#[cfg(feature = "desktop")]
pub use desktop::{context, run};
