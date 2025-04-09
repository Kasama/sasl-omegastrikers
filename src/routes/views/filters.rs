use crate::startgg::oauth::StartggUser;
use crate::startgg::tournaments::StartGGImage;

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
