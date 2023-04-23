use std::{path::PathBuf, sync::Mutex};

use serde::{Serialize, Deserialize};

use crate::unique::UniqueId;

/// Represents one unit of streamable data that can be shuffled to and from the disk
#[derive(Serialize, Deserialize, Debug)]
struct StreamingUnit<T> {
    index: usize,
    uid: UniqueId,
    path: PathBuf,
    data: Mutex<T>,
}

impl<T> StreamingUnit<T> {

}

struct StreamingIndex<T>(T) where T: PartialEq + Eq;

impl<T: PartialEq + Eq> StreamingIndex<T> {

}

struct Streaming {
    // a fixed unit size?
    // a fixed number of loaded units?
    // soft caps instead?
    // a priority queue of units that want to be loaded
    // a priority queue of units to be unloaded
    // heirarchical and spatial heuristics
    // can systems deal with not being able to load the data at that moment? notify when?
}

// should be able to just hand off data to the streaming system and it be mostly automatic
// need prediction to make it work smoothly?
struct StreamingInternal {

}

impl Streaming {
    pub fn request(uid: UniqueId) {
        // request data given a UID using the index part of the UID to reference the streaming unit the data belongs to
        // async?
    }
}

