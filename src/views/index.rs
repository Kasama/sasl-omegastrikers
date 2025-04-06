use askama::Template;

use crate::startgg::oauth::StartggUser;

use super::filters;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub maybe_user: Option<StartggUser>,
}
