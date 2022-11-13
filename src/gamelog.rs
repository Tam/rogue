use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct GameLog {
	pub entries : Vec<String>,
}