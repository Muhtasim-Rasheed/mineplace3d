use std::{collections::HashMap, path::PathBuf};

use mp3d_core::block::{BlockId, BlockStateMatcher};
use serde::de::Error;

use crate::resource::block::{model::BlockModelTransform, raw_model::RawBlockModelTransform};

#[derive(Debug, serde::Deserialize)]
struct StatesRaw {
    states: HashMap<String, StateDataRaw>,
}

#[derive(Debug, serde::Deserialize)]
struct StateDataRaw {
    model: String,
    transform: Option<RawBlockModelTransform>,
}

pub struct States {
    pub states: Vec<(BlockStateMatcher, StateData)>,
}

pub struct StateData {
    pub model: PathBuf,
    pub transform: Option<BlockModelTransform>,
}

impl States {
    pub fn load(block: BlockId, states_data: &str) -> Result<Self, serde_json::Error> {
        let states_raw: StatesRaw = serde_json::from_str(states_data)?;

        states_raw
            .states
            .into_iter()
            .map(|(key, value)| {
                let state = BlockStateMatcher::parse(block, &key)
                    .map_err(|e| format!("invalid blockstate matcher: {e}"))?;
                let model_path = PathBuf::from(format!("blocks/models/{}.json", value.model));
                Ok((
                    state,
                    StateData {
                        model: model_path,
                        transform: value.transform.map(BlockModelTransform::from),
                    },
                ))
            })
            .collect::<Result<_, _>>()
            .map(|v| Self { states: v })
            .map_err(|e: String| serde_json::Error::custom(e))
    }

    pub fn resolve(&self, state_data: u128) -> Option<&StateData> {
        self.states
            .iter()
            .find_map(|(matcher, data)| matcher.matches(state_data).then_some(data))
    }
}
