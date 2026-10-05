use std::{collections::HashMap, path::PathBuf};

use mp3d_core::block::{BlockId, BlockState};
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
    pub states: HashMap<u128, StateData>,
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
                let state =
                    parse_state_str(block, &key).map_err(|e| format!("invalid blockstate: {e}"))?;
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
}

fn parse_state_str(block: BlockId, state_str: &str) -> Result<u128, String> {
    let mut blockstate = BlockState::default_for(block);

    for part in state_str
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        let (name, val) = part
            .split_once('=')
            .ok_or_else(|| format!("expected '=' in property '{part}'"))?;
        let (name, val) = (name.trim(), val.trim());

        if !blockstate.set_str(name, val) {
            return Err(format!("unknown property or invalid value: {name}={val}"));
        }
    }

    Ok(blockstate.data)
}
