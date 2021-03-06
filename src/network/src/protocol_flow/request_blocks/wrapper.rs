/*
  Copyright (C) 2018-2020 The Purple Core Developers.
  This file is part of the Purple Core Library.

  The Purple Core Library is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  The Purple Core Library is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with the Purple Core Library. If not, see <http://www.gnu.org/licenses/>.
*/

use crate::bootstrap::cache::BootstrapCache;
use crate::protocol_flow::request_blocks::*;
use parking_lot::Mutex;
use triomphe::Arc;

#[derive(Clone, Debug)]
pub struct RequestBlocks {
    pub(crate) sender: Arc<Mutex<RequestBlocksSender>>,
    pub(crate) receiver: Arc<Mutex<RequestBlocksReceiver>>,
}

impl RequestBlocks {
    pub fn new() -> RequestBlocks {
        RequestBlocks {
            sender: Default::default(),
            receiver: Arc::new(Mutex::new(RequestBlocksReceiver::new())),
        }
    }
}
