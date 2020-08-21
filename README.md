# match-analysis
Match analyzer for League of Legends match data in CSV format used by Oracle's Elixir data aggregator.

## Building
This project uses Rust nightly, Rust can be installed with [rustup](https://rustup.rs/).

```
> git clone https://github.com/Eliasin/match-analysis.git
> cd match-analysis
> cargo build --release
```

## Usage
```
USAGE:
    match_analysis <matches> <query>
```

`matches` is a CSV formatted file in the format of [Oracle's Elixir](https://oracleselixir.com) data

`query` is a JSON file that specifies the query using a set of constraints and stats. An example is shown below.

```
{
    "constraints": [
        {
            "GameResult": true
        }
    ],
    "stats": [
        "Kills"
    ]
}
```

`constraints` is a list of constraints objects. You can constrain your query based on:
- Team Name [Team]
- Game Result (Winner/Loser) [GameResult]
- Side (Red/Blue) [Side]
- League (LCS/LFL/LCK ...) [League]

The name you should put in the constraints file is in square brackets.

Examples:

Query for stats of C9.
```
    "constraints": [
        {
            "Team": "C9"
        }
    ]
```

Query for winners in LCK.
```
    "constraints": [
        {
            "GameResult": true
        },
                {
            "League": "LCK"
        }
    ]
```

`stats` is a list of strings that contains the stats queried for. The stats are:
- Kills
- Deaths
- GoldDiff10
- GoldDiff15
- Barons
- FirstBaron
- Dragons
- FirstDragon
- Towers
- FirstTower
