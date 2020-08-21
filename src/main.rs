use clap::{ App, Arg };
use std::path::Path;
use std::fs::File;
use std::io::{BufReader};
use std::collections::HashMap;
use serde::Deserialize;
use std::error::Error;
use serde_json;
use csv;

#[derive(Deserialize, Clone, PartialEq)]
enum Sides {
    Red,
    Blue,
}

#[derive(Deserialize, Clone, PartialEq)]
enum Leagues {
    LFL,
    LCS,
    LCK,
    LPL,
    LEC,
    CK,
    VCS,
    LJL,
}

#[derive(Deserialize, Clone)]
enum Constraint {
    Team(String),
    GameResult(bool),
    Side(Sides),
    League(Leagues),
}

#[derive(Deserialize, Clone, Copy, Debug)]
enum Stats {
    Kills,
    Deaths,
    GoldDiff10,
    GoldDiff15,
    Barons,
    FirstBaron,
    Dragons,
    FirstDragon,
    Towers,
    FirstTower,
}

#[derive(Deserialize, Clone)]
struct Query {
    constraints: Vec<Constraint>,
    stats: Vec<Stats>,
}

type PlayerData = HashMap<String, String>;

#[derive(Debug, Clone)]
struct TeamData {
    name: String,
    data: HashMap<String, String>,
}

type GameID = String;
type GameData = (TeamData, Option<TeamData>);
type Games = HashMap<GameID, GameData>;

enum MergeType {
    And,
    Or,
    IntSum,
    NoOp,
}

fn get_player_data_game_id(player_data: &PlayerData) -> &str {
    player_data.get("gameid").unwrap()
}

fn get_player_data_team(player_data: &PlayerData) -> &str {
    player_data.get("team").unwrap()
}

fn player_row_to_player_data(player_row: &csv::StringRecord, header_legend: &HashMap<String, usize>) -> PlayerData {
    let mut player_data = PlayerData::new();
    for (attribute, index) in header_legend {
        let player_value = match player_row.get(*index) {
            Some(v) => v,
            None => continue
        };
        player_data.insert(attribute.clone(), player_value.to_string());
    }

    player_data
}

fn player_data_to_team_data(player_data: PlayerData) -> TeamData {
    TeamData {
        name: player_data.get("team").unwrap().clone(),
        data: player_data
    }
}

fn add_player_row_to_games(games: &mut Games, player_row: &csv::StringRecord, header_legend: &HashMap<String, usize>) {
    let player_data = player_row_to_player_data(player_row, header_legend);

    let game_id = get_player_data_game_id(&player_data).to_string();
    let team_data = player_data_to_team_data(player_data.clone());
    let player_team = get_player_data_team(&player_data);

    match games.get_mut(&game_id) {
        Some(teams) => {
            if teams.0.name == player_team {
                merge_player_data_into_team(&mut teams.0, &player_data);
            } else {
                teams.1 = Some(team_data);
            }
        },
        None => {
            games.insert(game_id, (team_data, None));
        }
    };

}

fn or_merge_values(a: &str, b: &str) -> String {
    if a == "1" || b == "1" {
        return "1".to_string();
    }
    "0".to_string()
}

fn and_merge_values(a: &str, b: &str) -> String {
    if a == "1" && b == "1" {
        return "1".to_string();
    }
    "0".to_string()
}

fn int_sum_merge_values(a: &str, b: &str) -> String {
    let a_int: i32 = str::parse(a).unwrap();
    let b_int: i32 = str::parse(b).unwrap();

    return (a_int + b_int).to_string();
}

fn op_merge_player_data_into_team(team_data: &mut TeamData, player_data: PlayerData, attribute: &String, op: &impl Fn(&str, &str) -> String) {
    let team_attribute = match team_data.data.get(attribute) {
        Some(v) => v.as_str(),
        None => ""
    };

    let player_attribute = match player_data.get(attribute) {
        Some(v) => v.as_str(),
        None => ""
    };

    let new = op(team_attribute, player_attribute);
    team_data.data.insert(attribute.clone(), new);
}

fn or_merge_player_data_into_team(team_data: &mut TeamData, player_data: PlayerData, attribute: &String) {
    op_merge_player_data_into_team(team_data, player_data, attribute, &or_merge_values);
}

fn int_sum_merge_player_data_into_team(team_data: &mut TeamData, player_data: PlayerData, attribute: &String) {
    op_merge_player_data_into_team(team_data, player_data, attribute, &int_sum_merge_values);
}

fn get_merge_type_of_attribute(attribute: &str) -> MergeType {
    match attribute {
        "firstbaron" => MergeType::Or,
        "firstblood" => MergeType::Or,
        "firsttower" => MergeType::Or,
        "kills" => MergeType::IntSum,
        "towers" => MergeType::IntSum,
        "barons" => MergeType::IntSum,
        "deaths" => MergeType::IntSum,
        "dragons" => MergeType::IntSum,
        "golddiffat10" => MergeType::IntSum,
        "golddiffat15" => MergeType::IntSum,
        _ => MergeType::NoOp,
    }
}

fn merge_attributes(a: &str, b: &str, merge_type: MergeType) -> String {
    match merge_type {
        MergeType::Or => or_merge_values(a, b),
        MergeType::IntSum => int_sum_merge_values(a, b),
        MergeType::And => and_merge_values(a, b),
        MergeType::NoOp => a.to_string(),
    }
}

fn merge_player_data_into_team(team_data: &mut TeamData, player_data: &PlayerData) {
    let merge_attributes = vec!["firstblood"];

    for attribute in merge_attributes.iter() {
        let merge_type = get_merge_type_of_attribute(attribute);
        match merge_type {
            MergeType::Or => or_merge_player_data_into_team(team_data, player_data.clone(), &attribute.to_string()),
            MergeType::IntSum => int_sum_merge_player_data_into_team(team_data, player_data.clone(), &attribute.to_string()),
            MergeType::NoOp => (),
            _ => ()
        }
    }

}

fn header_row_to_legend(header_row: csv::StringRecord) -> HashMap<String, usize> {
    let mut result = HashMap::new();
    for (index, attribute) in header_row.into_iter().enumerate() {
        result.insert(attribute.to_string(), index);
    }

    result
}

fn get_query_from_path(path: &Path) -> Result<Query, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let constraints = serde_json::from_reader(reader)?;
    Ok(constraints)
}

fn bool_to_result_str(b: bool) -> &'static str {
    match b {
        true => "1",
        false => "0"
    }
}

fn string_to_league(s: &str) -> Leagues {
    match s {
        "CK" => Leagues::CK,
        "LCK" => Leagues::LCK,
        "LCS" => Leagues::LCS,
        "LEC" => Leagues::LEC,
        "LFL" => Leagues::LFL,
        "LJL" => Leagues::LJL,
        "LPL" => Leagues::LPL,
        "VCS" => Leagues::VCS,
        _ => panic!("Unrecognized league: {:?}", s)
    }
}

fn string_to_side(s: &str) -> Sides {
    match s {
        "Blue" => Sides::Blue,
        "Red" => Sides::Red,
        _ => panic!("Unrecognized side: {:?}", s)
    }
}

fn fits_constraint(team: &TeamData, constraint: &Constraint) -> bool {
    match constraint {
        Constraint::GameResult(result) => {
            return team.data.get("result").unwrap() == bool_to_result_str(*result);
        },
        Constraint::Team(name) => {
            return team.name == *name;
        },
        Constraint::League(league) => {
            return string_to_league(team.data.get("league").unwrap()) == *league;
        },
        Constraint::Side(side) => {
            return string_to_side(team.data.get("side").unwrap()) == *side;
        }
    }
}

fn fits_constraints(team: &TeamData, constraints: &Vec<Constraint>) -> bool {
    for constraint in constraints {
        if !fits_constraint(team, constraint) {
            return false;
        }
    }

    return true;
}

fn stat_to_attribute_string(stat: Stats) -> &'static str {
    match stat {
        Stats::Barons => "barons",
        Stats::Deaths => "deaths",
        Stats::Dragons => "dragons",
        Stats::FirstBaron => "firstbaron",
        Stats::FirstDragon => "firstdragon",
        Stats::FirstTower => "firsttower",
        Stats::GoldDiff10 => "golddiffat10",
        Stats::GoldDiff15 => "golddiffat15",
        Stats::Kills => "kills",
        Stats::Towers => "towers",
    }
}

fn query_stat(stat: Stats, team: &TeamData) -> String {
    match stat {
        Stats::Kills => {
            return team.data.get("kills").unwrap().clone();
        },
        _ => panic!("Unknown stat: {:?}", stat)
    }
}

fn query_games(query: Query, games: Games) -> HashMap<String, String> {
    let mut results = HashMap::<String, String>::new();
    for (_, game) in games {
        let team_a = game.0;
        let team_b = match game.1 {
            Some(v) => v,
            None => continue
        };

        for team in [team_a, team_b].iter() {

            if fits_constraints(team, &query.constraints) {

                for stat in &query.stats {
                    let stat_attribute_string = stat_to_attribute_string(*stat);
                    let merge_type = get_merge_type_of_attribute(stat_attribute_string);

                    let default = String::from("0");
                    let current = match results.get(stat_attribute_string) {
                        Some(v) => v,
                        None => &default,
                    };
                    let v = query_stat(*stat, team);

                    let new = merge_attributes(&current.to_string(), &v, merge_type);

                    results.insert(stat_attribute_string.to_string(), new);
                }

            }

        }

    }

    results
}

fn main() {
	  let matches = App::new("Match Data Analyzer")
        .version("1.0")
        .author("Steven Pham")
        .about("Compiles data about matchsets")
        .arg(Arg::with_name("matches")
             .help("Path to matches CSV file")
             .takes_value(true)
             .required(true))
        .arg(Arg::with_name("query")
             .help("Path to query file")
             .takes_value(true)
             .required(true))
        .get_matches();

    let matches_path = matches.value_of("matches").unwrap();
    let query_path = matches.value_of("query").unwrap();

    let query = match get_query_from_path(Path::new(query_path)) {
        Ok(v) => v,
        Err(error) => panic!("Query could not be parsed: {:?}", error)
    };

    let mut reader = match csv::Reader::from_path(Path::new(matches_path)) {
        Ok(v) => v,
        Err(error) => panic!("Reader could not be created: {:?}", error)
    };

    let header_legend = match reader.headers() {
        Ok(v) => header_row_to_legend(v.clone()),
        Err(error) => panic!("No headers in matches: {:?}", error)
    };

    let mut games = Games::new();
    for record in reader.into_records() {
        let row = match record {
            Ok(v) => v,
            Err(_) => continue
        };

        add_player_row_to_games(&mut games, &row, &header_legend);
    }

    let results = query_games(query, games);
    println!("{:?}", results);

}
