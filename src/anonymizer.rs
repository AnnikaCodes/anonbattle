/// Contains the core anonymization logic

use std::collections::HashMap;
use std::str::FromStr;

use regex::Regex;
use lazy_static::*;

use serde_json::{Value, json};

lazy_static! {
    static ref ID_REGEX: Regex = Regex::new(r"[^A-Za-z0-9]").unwrap();
    static ref INPUTLOG_ANONYMIZER_REGEX: Regex = Regex::new(r#"name":".*","#).unwrap();
}

fn to_id(str: &str) -> String {
    (*ID_REGEX.replace_all(str, "")).to_lowercase()
}

/// Tracks players
struct PlayerTracker {
    players: HashMap<String, String>,
    cur_player_number: i32,
}

impl PlayerTracker {
    fn new() -> PlayerTracker {
        PlayerTracker {
            players: HashMap::new(),
            cur_player_number: 0,
        }
    }

    fn anonymize(&mut self, userid: String) -> String {
        match self.players.get(&userid) {
            Some(anonymized) => anonymized.to_string(),
            None => {
                self.cur_player_number += 1;
                let num = self.cur_player_number;
                self.players.insert(userid, num.to_string());
                self.cur_player_number.to_string()
            }
        }
    }
}

/// Anonymizes string JSON while tracking state
pub struct Anonymizer {
    players: PlayerTracker,
    current_battle_number: u32,
    /// Panics if player names sneak past
    is_safe: bool,
}

impl Anonymizer {
    pub fn new() -> Anonymizer {
        Anonymizer {
            players: PlayerTracker::new(),
            current_battle_number: 0,
            is_safe: false,
        }
    }

    /// Anonymizes a log.
    ///
    /// Returns a tuple: (json, battle_number)
    pub fn anonymize(&mut self, raw: &str) -> (String, u32) {
        let json: serde_json::Map<String, Value> = serde_json::from_str(raw).unwrap();

        let p1 = json["p1"].as_str().unwrap();
        let p2 = json["p2"].as_str().unwrap();
        let p1_id = to_id(p1);
        let p2_id = to_id(p2);

        let p1_anon = self.players.anonymize(p1.to_string());
        let p2_anon = self.players.anonymize(p2.to_string());

        let mut json_result = json.clone();
        // Anonnymize
        json_result["p1"] = Value::String(p1_anon.clone());
        json_result["p2"] = Value::String(p2_anon.clone());
        json_result["winner"] = Value::String(self.players.anonymize(json["winner"].as_str().unwrap().to_owned()));

        json_result["p1rating"] = Value::Null;
        json_result["p2rating"] = Value::Null;
        json_result["roomid"] = Value::Null;

        // "Sat Nov 21 2020 17:05:04 GMT-0500 (Eastern Standard Time)" -> "Sat Nov 21 2020 17"
        let mut timestamp = json["timestamp"]
            .as_str()
            .unwrap()
            .split(':')
            .collect::<Vec<&str>>()[0]
            .to_owned();
        timestamp.push_str(":XX");

        json_result["timestamp"] = json!(timestamp);


        let il = json["inputLog"].as_array().unwrap().iter();
        json_result["inputLog"] = serde_json::json!(il.filter_map(|inputlog_part| {
            let inputlog_part_string: &str = inputlog_part.as_str().unwrap();

            if inputlog_part_string.starts_with(">player p1") {
                let res = INPUTLOG_ANONYMIZER_REGEX.replace_all(inputlog_part_string, |_: &regex::Captures| {
                    format!("\"name\":\"{}\",", p1_anon)
                });
                return Some(json!(res));
            } else if inputlog_part_string.starts_with(">player p2") {
                let res = INPUTLOG_ANONYMIZER_REGEX.replace_all(inputlog_part_string, |_: &regex::Captures| {
                    format!("\"name\":\"{}\",", p2_anon)
                });
                return Some(json!(res));
            } else if inputlog_part_string.starts_with(">chat ") {
                return None;
            }

            Some(inputlog_part.clone())
        }).collect::<Vec<serde_json::Value>>());

        let log = json["log"].as_array().unwrap().iter();
        json_result["log"] = serde_json::json!(log.filter_map(|log_part| {
            let log_part_string: &str = log_part.as_str().unwrap();

            // Remove chat and timers (privacy threat)
            if log_part_string.starts_with("|c|") || log_part_string.starts_with("|c:|") || log_part_string.starts_with("|inactive|") {
                return None;
            }

            if log_part_string.starts_with("|j|") ||
                log_part_string.starts_with("|J|") ||

                log_part_string.starts_with("|l|") ||
                log_part_string.starts_with("|L|") ||

                log_part_string.starts_with("|N|") ||
                log_part_string.starts_with("|n|") ||

                log_part_string.starts_with("|win|") ||
                log_part_string.starts_with("|tie|") ||
                log_part_string.starts_with("|-message|") ||
                log_part_string.starts_with("|raw|") ||
                log_part_string.starts_with("|player|")
            {
                return Some(json!(log_part_string
                    .replace(p1, &p1_anon)
                    .replace(p2, &p2_anon)
                    .replace(&p1_id, &p1_anon)
                    .replace(&p2_id, &p2_anon)
                ));
            }
            let p1regex = Regex::from_str(&["\\|p1[ab]?: (", &regex::escape(p1), "|", &regex::escape(&p1_id), ")"].join("")).unwrap();
            let p2regex = Regex::from_str(&["\\|p2[ab]?: (", &regex::escape(p2), "|", &regex::escape(&p2_id), ")"].join("")).unwrap();

            return Some(json!(p2regex.replace_all(p1regex.replace_all(log_part_string, &p1_anon as &str).as_ref(), &p2_anon as &str)));
        }).collect::<Vec<serde_json::Value>>());

        self.current_battle_number += 1;
        let result = serde_json::to_string(&json_result).unwrap();

        if result.contains(p1) || result.contains(&p1_id) || result.contains(p2) || result.contains(&p2_id) {
            println!("{}", json["roomid"]);
            if self.is_safe {
                panic!("Failure");
            }
        }

        (result, self.current_battle_number)
    }
}
