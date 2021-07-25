#![feature(option_result_unwrap_unchecked)]
#![feature(slice_as_chunks)]
#![feature(once_cell)]

mod cmd_addpremium;
mod cmd_credits;
mod cmd_donate;
pub mod cmd_error;
mod cmd_eval;
mod cmd_getkey;
mod cmd_help;
mod cmd_info;
mod cmd_join;
mod cmd_ping;
mod cmd_prefix;
mod cmd_rejoinall;
mod cmd_setup;
mod cmd_shutdown;
mod cmd_stats;
mod cmd_template;
pub mod groups;

pub use cmd_addpremium::*;
pub use cmd_credits::*;
pub use cmd_donate::*;
pub use cmd_error::*;
pub use cmd_eval::*;
pub use cmd_getkey::*;
pub use cmd_help::*;
pub use cmd_info::*;
pub use cmd_join::*;
pub use cmd_ping::*;
pub use cmd_prefix::*;
pub use cmd_rejoinall::*;
pub use cmd_setup::*;
pub use cmd_shutdown::*;
pub use cmd_stats::*;
pub use groups::*;
// not a real command
// pub use cmd_template::*;
