/*
 * Copyright (c) 2020.  Shoyo Inokuchi.
 * Please refer to github.com/shoyo/jin for more information about this project and its license.
 */

use super::policy::Policy;
use crate::common::constants::BufferFrameIdT;

/// An LRU eviction policy for the database buffer.
pub struct LRUPolicy {}

impl Policy for LRUPolicy {
    fn new() -> Self {
        Self {}
    }

    fn evict(&mut self) -> Result<BufferFrameIdT, String> {
        todo!()
    }

    fn pin(&mut self, frame_id: BufferFrameIdT) {
        todo!()
    }

    fn unpin(&mut self, frame_id: BufferFrameIdT) {
        todo!()
    }
}