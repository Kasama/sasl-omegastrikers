use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum OmegaStrikersPlayerStatus {
    Offline,
    Online,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmegaStrikersPlatformDiscord {
    pub discord_id: String,
    pub has_full_account: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct OmegaStrikersPlatformIds {
    pub discord: Option<OmegaStrikersPlatformDiscord>,
    pub playstation_id: Option<String>,
    pub xuid: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmegaStrikersOrganization {
    pub organization_id: String,
    pub logo_id: String,
    pub name: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmegaStrikersUser {
    pub username: String,
    pub player_id: String,
    pub logo_id: String,
    pub title: String,
    pub nameplate_id: String,
    pub emoticon_id: String,
    pub title_id: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub platform_ids: OmegaStrikersPlatformIds,
    pub mastery_level: u32,
    pub player_status: OmegaStrikersPlayerStatus,
    pub organization: Option<OmegaStrikersOrganization>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_desserialize() {
        let s = r#"{
      "username": ".nell.",
      "playerId": "644ae73c4ed4435bfeac6e12",
      "logoId": "UNUSED",
      "title": "ProLeagueR3DemonDrive",
      "nameplateId": "NameplateData_ClarionCorpMagicShow",
      "emoticonId": "EmoticonData_AiMiJam",
      "titleId": "TitleData_VerifiedFortress",
      "tags": [],
      "platformIds": {
        "playstationId": "4390652780776029962",
        "discord": {
          "discordId": "612471247523020830",
          "hasFullAccount": true
        }
      },
      "masteryLevel": 747,
      "playerStatus": "Offline",
      "organization": {
        "organizationId": "653a799d5c84410dc0eaea5f",
        "logoId": "UNUSED",
        "name": "ProLeagueR3DemonDrive"
      }
    }"#;

        let o: OmegaStrikersUser = serde_json::from_str(s)
            .inspect_err(|e| panic!("Failed to decode json: {e:?}"))
            .unwrap();

        panic!("{:?}", o);
    }
}
