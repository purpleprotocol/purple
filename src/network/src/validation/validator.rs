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
use crate::protocol_flow::ping_pong::PingPong;
use crate::protocol_flow::request_blocks::RequestBlocks;
use crate::protocol_flow::request_peers::RequestPeers;
use crate::protocol_flow::transaction_propagation::TransactionPropagation;
use std::default::Default;

#[derive(Clone, Debug)]
/// Struct wrapping all protocol flows. This
/// is instantiated once per each connected peer.
pub struct ProtocolValidator {
    /// Ping/Pong protocol flow
    pub(crate) ping_pong: PingPong,

    /// Request blocks protocol flow
    pub(crate) request_blocks: RequestBlocks,

    /// Request peers protocol flow
    pub(crate) request_peers: RequestPeers,

    /// Transaction propagation protocol flow
    pub(crate) transaction_propagation: TransactionPropagation,
}

impl ProtocolValidator {
    pub fn new(bootstrap_cache: BootstrapCache) -> ProtocolValidator {
        ProtocolValidator {
            ping_pong: Default::default(),
            request_blocks: RequestBlocks::new(),
            request_peers: RequestPeers::new(bootstrap_cache),
            transaction_propagation: Default::default(),
        }
    }
}
