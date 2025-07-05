use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AvatarViewModel {
    pub url: String,
    pub dominant_color_r: Option<u32>,
    pub dominant_color_g: Option<u32>,
    pub dominant_color_b: Option<u32>
}

impl AvatarViewModel {
    pub fn new(url: String) -> Self {
        let mut avatar = Self {
            url: url.clone(),
            dominant_color_r: None,
            dominant_color_g: None,
            dominant_color_b: None
        };

        if let Some(query_start) = url.find('?') {
            let query_part = &url[query_start + 1..];

            for param in query_part.split('&') {
                if let Some(eq_pos) = param.find('=') {
                    let key = &param[0..eq_pos];
                    let value = &param[eq_pos + 1..];

                    match key {
                        "r" => avatar.dominant_color_r = value.parse::<u32>().ok(),
                        "g" => avatar.dominant_color_g = value.parse::<u32>().ok(),
                        "b" => avatar.dominant_color_b = value.parse::<u32>().ok(),
                        _ => {}
                    }
                }
            }
        }

        avatar
    }
}
