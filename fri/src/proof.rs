use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FriProofLayer {
    pub values: Vec<Vec<u8>>,
    pub paths: Vec<Vec<[u8; 32]>>,
    pub depth: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FriProof {
    pub layers: Vec<FriProofLayer>,
    pub rem_values: Vec<u8>,
}
