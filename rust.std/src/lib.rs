pub mod serenity_backport;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
