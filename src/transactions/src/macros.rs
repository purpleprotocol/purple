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

macro_rules! impl_hash {
    () => {
        /// Computes the transaction's hash.
        pub fn compute_hash(&mut self) {
            // Assemble data
            let message = assemble_message(&self);

            // Hash data
            let hash = crypto::hash_slice(&message);

            self.hash = Some(hash);
        }

        /// Computes the message that is passed to the
        /// hash function of this transaction.
        pub fn compute_hash_message(&self) -> Vec<u8> {
            assemble_message(&self)
        }

        /// Verifies the correctness of the hash of the transaction.
        ///
        /// This function will panic if the hash field is missing.
        pub fn verify_hash(&mut self) -> bool {
            let hash = if let Some(hash) = &self.hash {
                hash.0
            } else {
                panic!("Hash field is missing!");
            };

            let oracle_message = assemble_message(&self);
            let oracle_hash = crypto::hash_slice(&oracle_message);

            hash == oracle_hash.0
        }
    };
}
