pub mod ar_event;
pub mod data;
pub mod member_permission_calc;
pub mod punishments;
pub mod stings;
pub mod templates;

pub type Error = Box<dyn std::error::Error + Send + Sync>; // This is constant and should be copy pasted
