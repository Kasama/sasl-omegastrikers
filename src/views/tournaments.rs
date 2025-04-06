use askama::Template;

use crate::startgg::oauth::StartggUser;
use crate::startgg::tournaments::Tournament;

use super::filters;

#[derive(Template)]
#[template(path = "tournaments.html")]
pub struct TournamentsTemplate {
    pub maybe_user: Option<StartggUser>,
    pub tournaments: Vec<Tournament>,
}
