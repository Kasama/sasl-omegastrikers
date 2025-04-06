use crate::database::user::User;

#[derive(Debug)]
pub struct Group {
    pub extras: Vec<User>,
    pub forwards: (User, User),
    pub goalie: User,
    pub coach: User,
}
