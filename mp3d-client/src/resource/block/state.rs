use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, serde::Deserialize)]
struct StatesRaw {
    states: HashMap<String, StateDataRaw>,
}

#[derive(Debug, serde::Deserialize)]
struct StateDataRaw {
    model: String,
}

pub struct States {
    pub states: HashMap<u16, StateData>,
}

pub struct StateData {
    pub model: PathBuf,
}

impl States {
    pub fn load(states_data: &str) -> Result<Self, serde_json::Error> {
        let states_raw: StatesRaw = serde_json::from_str(&states_data)?;

        let states = states_raw
            .states
            .into_iter()
            .map(|(key, value)| {
                let state_type = u16::from_str_radix(&key, 16).unwrap();
                let model_path = PathBuf::from(format!("blocks/models/{}.json", value.model));
                (state_type, StateData { model: model_path })
            })
            .collect();

        Ok(Self { states })
    }
}
