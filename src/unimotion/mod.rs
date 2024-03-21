use crate::prelude::*;

pub mod device;
// pub use manager::{UnimotionManager, UNIMOTION_RECEIVER};
pub use manager::{UnimotionManager, Command};

use std::fmt::{Debug, Formatter};
use std::sync::Arc;

mod manager;
