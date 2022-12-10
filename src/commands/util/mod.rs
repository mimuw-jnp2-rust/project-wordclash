pub mod errors;
pub mod queries;
pub mod ctxmanager;
pub use errors::{
    CmdError,
    CmdResult
};
pub use queries::{
    ensure_word,
};