use serenity::all::CreateCommand;

pub mod register;

pub fn register_all() -> Vec<CreateCommand> {
    vec![register::register()]
}
