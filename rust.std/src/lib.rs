pub mod objectstore;
pub mod priorityset;
pub mod serenity_backport;
pub mod utils;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
