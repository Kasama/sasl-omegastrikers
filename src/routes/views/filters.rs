use uuid::Uuid;

use crate::startgg::oauth::StartggUser;
use crate::startgg::tournaments::{StartGGImage, StartGGTeam};

pub fn get_smallest_image(images: &[StartGGImage]) -> askama::Result<Option<&StartGGImage>> {
    Ok(images.iter().reduce(|im, ne| {
        if im.width * im.height > ne.width * ne.height {
            ne
        } else {
            im
        }
    }))
}

pub fn user_display_name(user: &StartggUser) -> askama::Result<&str> {
    Ok(if let Some(ref name) = user.gamer_tag {
        name
    } else {
        &user.slug
    })
}

pub fn team_display_name(team: &StartGGTeam) -> askama::Result<&str> {
    Ok(if let Some(ref nickname) = team.nickname {
        if nickname.is_empty() {
            &team.name
        } else {
            nickname
        }
    } else {
        &team.name
    })
}

pub fn team_full_name(team: &StartGGTeam) -> askama::Result<String> {
    Ok(if let Some(ref nickname) = team.nickname {
        if nickname.is_empty() {
            team.name.clone()
        } else {
            format!("{} ({})", team.name, nickname)
        }
    } else {
        team.name.clone()
    })
}

fn uuid_to_vdo_id(uuid: &Uuid) -> askama::Result<String> {
    let mut uuid_str = uuid.simple().to_string();
    uuid_str.remove(12); // UUIDv4 version number
    uuid_str.remove(15); // UUIDv4 variant
    Ok(uuid_str)
}

pub fn vdo_invite_link(uuid: &Uuid, kind: &str) -> askama::Result<String> {
    Ok(format!(
        "https://vdo.ninja/?room={}&vd=0&avatar&push={}",
        uuid_to_vdo_id(uuid)?,
        kind
    ))
}

pub fn vdo_view_link(uuid: &Uuid, kind: &str) -> askama::Result<String> {
    Ok(format!(
        "https://vdo.ninja/?view={}&solo&room={}",
        kind,
        uuid_to_vdo_id(uuid)?
    ))
}

pub fn vdo_director_link(uuid: &Uuid) -> askama::Result<String> {
    Ok(format!(
        "https://vdo.ninja/?director={}",
        uuid_to_vdo_id(uuid)?
    ))
}

pub fn duration_text(duration: &chrono::Duration) -> askama::Result<Option<String>> {
    if duration <= &chrono::Duration::zero() {
        return Ok(None);
    }
    let total_secs = duration.num_seconds();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    let mut result = String::new();

    if hours > 0 {
        result.push_str(&format!("{}h", hours));
    }

    if minutes > 0 || (hours > 0 && seconds > 0) {
        result.push_str(&format!("{}m", minutes));
    }

    if seconds > 0 || result.is_empty() {
        result.push_str(&format!("{}s", seconds));
    }

    Ok(Some(result))
}

pub fn datetime_format(dt: &chrono::DateTime<chrono::FixedOffset>) -> askama::Result<String> {
    Ok(dt.format("%Y-%m-%dT%H:%M:%S").to_string())
}
