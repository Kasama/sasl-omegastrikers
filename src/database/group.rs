use crate::database::user::User;

#[derive(Debug)]
pub struct Group {
    pub members: (User, Option<User>, Option<User>),
}
