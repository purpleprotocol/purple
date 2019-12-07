/*
  Copyright (C) 2018-2019 The Purple Core Developers.
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

use account::{Address, Balance, NormalAddress};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use crypto::{Hash, PublicKey as Pk, SecretKey as Sk, Signature};
use patricia_trie::{TrieDBMut, TrieMut};
use persistence::{BlakeDbHasher, Codec};
use std::io::Cursor;
use crate::CreateMintable;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ChangeMinter {
    /// The current minter
    pub(crate) minter: NormalAddress,

    /// The address of the new minter
    pub(crate) new_minter: Address,

    /// The global identifier of the mintable asset
    pub(crate) asset_hash: Hash,

    /// The global identifier of the asset in which
    /// the transaction fee is paid in.
    pub(crate) fee_hash: Hash,

    /// The transaction's fee
    pub(crate) fee: Balance,

    /// Nonce
    pub(crate) nonce: u64,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) hash: Option<Hash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) signature: Option<Signature>,
}

impl ChangeMinter {
    pub const TX_TYPE: u8 = 8;

    /// Validates the transaction against the provided state.
    pub fn validate(&self, trie: &TrieDBMut<BlakeDbHasher, Codec>) -> bool {
        let zero = Balance::from_bytes(b"0.0").unwrap();

        if !self.verify_sig() {
            return false;
        }

        let bin_minter = &self.minter.to_bytes();
        let bin_fee_hash = &self.fee_hash.to_vec();

        // Convert address to strings
        let minter = hex::encode(bin_minter);

        // Convert hashes to strings
        let fee_hash = hex::encode(bin_fee_hash);

        // Calculate nonce keys
        //
        // The key of a nonce has the following format:
        // `<account-address>.n`
        let minter_nonce_key = format!("{}.n", minter);
        let minter_nonce_key = minter_nonce_key.as_bytes();

        // Calculate currency keys
        //
        // The key of a currency entry has the following format:
        // `<account-address>.<currency-hash>`
        let fee_key = format!("{}.{}", minter, fee_hash);

        // Retrieve serialized nonce
        let bin_nonce = match trie.get(&minter_nonce_key) {
            Ok(Some(nonce)) => nonce,
            Ok(None) => return false,
            Err(err) => panic!(err),
        };

        let stored_nonce = decode_be_u64!(bin_nonce).unwrap();
        if stored_nonce + 1 != self.nonce {
            return false;
        }

        let mut minter_fee_balance = unwrap!(
            Balance::from_bytes(&unwrap!(
                trie.get(&fee_key.as_bytes()).unwrap(),
                "The minter does not have an entry for the given currency"
            )),
            "Invalid stored balance format"
        );

        // Subtract fee from minter balance
        minter_fee_balance -= self.fee.clone();

        minter_fee_balance >= zero
    }

    /// Applies the change minter transaction to the provided database.
    ///
    /// # Remarks
    ///
    /// It panics if the minter address doesn't exist
    pub fn apply(&self, trie: &mut TrieDBMut<BlakeDbHasher, Codec>) {
        let bin_minter = &self.minter.to_bytes();
        let bin_new_minter = &self.new_minter.to_bytes();
        let bin_asset_hash = &self.asset_hash.to_vec();
        let bin_fee_hash = &self.fee_hash.to_vec();

        if bin_minter == bin_new_minter {
            panic!("The new address of the minter should be different from the current!");
        }

        // Convert addresses to strings
        let minter = hex::encode(bin_minter);
        let new_minter = hex::encode(bin_new_minter);

        // Convert hashes to strings
        let asset_hash = hex::encode(bin_asset_hash);
        let fee_hash = hex::encode(bin_fee_hash);

        // Calculate nonce keys
        //
        // The key of a nonce has the following format:
        // `<account-address>.n`
        let minter_nonce_key = format!("{}.n", minter);
        let minter_nonce_key = minter_nonce_key.as_bytes();
        let new_minter_nonce_key = format!("{}.n", new_minter);
        let new_minter_nonce_key = new_minter_nonce_key.as_bytes();

        // Handle nonce
        // Retrieve serialized nonce
        let bin_minter_nonce = &trie.get(&minter_nonce_key).unwrap().unwrap();
        let bin_new_minter_nonce = trie.get(&new_minter_nonce_key);

        let mut nonce_rdr = Cursor::new(bin_minter_nonce);

        // Read the nonce of the minter
        let mut nonce = nonce_rdr.read_u64::<BigEndian>().unwrap();

        // Increment minter nonce
        nonce += 1;

        // Create nonce buffer
        let mut nonce_buf: Vec<u8> = Vec::with_capacity(8);

        // Write new nonce to buffer
        nonce_buf.write_u64::<BigEndian>(nonce).unwrap();

        // Calculate minter address key
        //
        // The key of a currency's minter address has the following format:
        // `<currency-hash>.m`
        let asset_hash_minter_key = format!("{}.m", asset_hash);
        let asset_hash_minter_key = asset_hash_minter_key.as_bytes();

        // Calculate currency keys
        //
        // The key of a currency entry has the following format:
        // `<account-address>.<currency-hash>`
        let minter_fee_key = format!("{}.{}", minter, fee_hash);
        let minter_fee_key = minter_fee_key.as_bytes();

        match bin_new_minter_nonce {
            // The new minter account exists
            Ok(Some(_)) => {
                let mut minter_fee_balance = unwrap!(
                    Balance::from_bytes(&unwrap!(
                        trie.get(&minter_fee_key).unwrap(),
                        "The minter does not have an entry for the given currency"
                    )),
                    "Invalid stored balance format"
                );

                // Subtract fee from minter balance
                minter_fee_balance -= self.fee.clone();

                // Update trie
                trie.insert(asset_hash_minter_key, &bin_new_minter).unwrap();
                trie.insert(&minter_nonce_key, &nonce_buf).unwrap();
                trie.insert(&minter_fee_key, &minter_fee_balance.to_bytes())
                    .unwrap();
            }
            // The new minter account doesn't exist, so we create it
            Ok(None) => {
                let mut minter_fee_balance = unwrap!(
                    Balance::from_bytes(&unwrap!(
                        trie.get(&minter_fee_key).unwrap(),
                        "The minter does not have an entry for the given currency"
                    )),
                    "Invalid stored balance format"
                );

                // Subtract fee from minter balance
                minter_fee_balance -= self.fee.clone();

                // Update trie
                trie.insert(&minter_nonce_key, &nonce_buf).unwrap();
                trie.insert(&new_minter_nonce_key, &[0, 0, 0, 0, 0, 0, 0, 0])
                    .unwrap();
                trie.insert(asset_hash_minter_key, &bin_new_minter).unwrap();
                trie.insert(&minter_fee_key, &minter_fee_balance.to_bytes())
                    .unwrap();
            }
            Err(err) => panic!(err),
        }
    }

    /// Signs the transaction with the given secret key.
    pub fn sign(&mut self, skey: Sk) {
        // Assemble data
        let message = assemble_message(&self);

        // Sign data
        let signature = crypto::sign(&message, &skey);
        self.signature = Some(signature);
    }

    /// Verifies the signature of the transaction.
    ///
    /// Returns `false` if the signature field is missing.
    pub fn verify_sig(&self) -> bool {
        let message = assemble_message(&self);

        match self.signature {
            Some(ref sig) => crypto::verify(&message, sig, &self.minter.pkey()),
            None => false,
        }
    }

    /// Serializes the transaction struct to a binary format.
    ///
    /// Fields:
    /// 1) Transaction type(8)  - 8bits
    /// 2) Fee length           - 8bits
    /// 3) Nonce                - 64bits
    /// 4) Minter               - 33byte binary
    /// 5) New Minter           - 33byte binary
    /// 6) Asset hash           - 32byte binary
    /// 7) Fee hash             - 32byte binary
    /// 8) Signature            - 64byte binary
    /// 9) Fee                  - Binary of fee length
    pub fn to_bytes(&self) -> Result<Vec<u8>, &'static str> {
        let mut buf: Vec<u8> = Vec::new();

        let mut signature = if let Some(signature) = &self.signature {
            signature.to_bytes()
        } else {
            return Err("Signature field is missing");
        };

        let tx_type: u8 = Self::TX_TYPE;
        let minter = &self.minter.to_bytes();
        let new_minter = &self.new_minter.to_bytes();
        let asset_hash = &&self.asset_hash.0;
        let fee_hash = &&self.fee_hash.0;
        let fee = &self.fee.to_bytes();
        let fee_len = fee.len();
        let nonce = &self.nonce;

        // Write to buffer
        buf.write_u8(tx_type).unwrap();
        buf.write_u8(fee_len as u8).unwrap();
        buf.write_u64::<BigEndian>(*nonce).unwrap();

        buf.append(&mut minter.to_vec());
        buf.append(&mut new_minter.to_vec());
        buf.append(&mut asset_hash.to_vec());
        buf.append(&mut fee_hash.to_vec());
        buf.append(&mut signature);
        buf.append(&mut fee.to_vec());

        Ok(buf)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<ChangeMinter, &'static str> {
        let mut rdr = Cursor::new(bytes.to_vec());
        let tx_type = if let Ok(result) = rdr.read_u8() {
            result
        } else {
            return Err("Bad transaction type");
        };

        if tx_type != Self::TX_TYPE {
            return Err("Bad transation type");
        }

        rdr.set_position(1);

        let fee_len = if let Ok(result) = rdr.read_u8() {
            result
        } else {
            return Err("Bad fee len");
        };

        rdr.set_position(2);

        let nonce = if let Ok(result) = rdr.read_u64::<BigEndian>() {
            result
        } else {
            return Err("Bad nonce");
        };

        let mut buf: Vec<u8> = rdr.into_inner();
        let _: Vec<u8> = buf.drain(..10).collect();

        let minter = if buf.len() > 33 as usize {
            let minter_vec: Vec<u8> = buf.drain(..33).collect();

            match NormalAddress::from_bytes(&minter_vec) {
                Ok(addr) => addr,
                Err(err) => return Err(err),
            }
        } else {
            return Err("Incorrect packet structure");
        };

        let new_minter = if buf.len() > 33 as usize {
            let new_minter_vec: Vec<u8> = buf.drain(..33).collect();

            match Address::from_bytes(&new_minter_vec) {
                Ok(addr) => addr,
                Err(err) => return Err(err),
            }
        } else {
            return Err("Incorrect packet structure");
        };

        let asset_hash = if buf.len() > 32 as usize {
            let mut hash = [0; 32];
            let hash_vec: Vec<u8> = buf.drain(..32).collect();

            hash.copy_from_slice(&hash_vec);

            Hash(hash)
        } else {
            return Err("Incorrect packet structure");
        };

        let fee_hash = if buf.len() > 32 as usize {
            let mut hash = [0; 32];
            let hash_vec: Vec<u8> = buf.drain(..32).collect();

            hash.copy_from_slice(&hash_vec);

            Hash(hash)
        } else {
            return Err("Incorrect packet structure");
        };

        let signature = if buf.len() > 64 as usize {
            let sig_vec: Vec<u8> = buf.drain(..64).collect();

            match Signature::from_bytes(&sig_vec) {
                Ok(sig) => sig,
                Err(_) => return Err("Bad signature"),
            }
        } else {
            return Err("Incorrect packet structure");
        };

        let fee = if buf.len() == fee_len as usize {
            let fee_vec: Vec<u8> = buf.drain(..fee_len as usize).collect();

            match Balance::from_bytes(&fee_vec) {
                Ok(result) => result,
                Err(_) => return Err("Bad gas price"),
            }
        } else {
            return Err("Incorrect packet structure");
        };

        let mut change_minter = ChangeMinter {
            minter: minter,
            new_minter: new_minter,
            asset_hash: asset_hash,
            fee_hash: fee_hash,
            fee: fee,
            nonce: nonce,
            hash: None,
            signature: Some(signature),
        };

        change_minter.compute_hash();
        Ok(change_minter)
    }

    /// Returns a random valid transaction for the provided state.
    pub fn arbitrary_valid(trie: &mut TrieDBMut<BlakeDbHasher, Codec>, sk: Sk) -> Self {
        unimplemented!();
    }

    impl_hash!();
}

fn assemble_message(obj: &ChangeMinter) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let mut minter = obj.minter.to_bytes();
    let mut new_minter = obj.new_minter.to_bytes();
    let mut fee = obj.fee.to_bytes();
    let asset_hash = obj.asset_hash.0;
    let fee_hash = obj.fee_hash.0;

    // Compose data to hash
    buf.append(&mut minter);
    buf.append(&mut new_minter);
    buf.append(&mut asset_hash.to_vec());
    buf.append(&mut fee_hash.to_vec());
    buf.append(&mut fee);

    buf
}

use quickcheck::Arbitrary;

impl Arbitrary for ChangeMinter {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> ChangeMinter {
        let mut tx = ChangeMinter {
            minter: Arbitrary::arbitrary(g),
            new_minter: Arbitrary::arbitrary(g),
            asset_hash: Arbitrary::arbitrary(g),
            fee_hash: Arbitrary::arbitrary(g),
            fee: Arbitrary::arbitrary(g),
            nonce: Arbitrary::arbitrary(g),
            hash: None,
            signature: Some(Arbitrary::arbitrary(g)),
        };

        tx.compute_hash();
        tx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use account::NormalAddress;
    use crypto::Identity;

    #[test]
    fn validate() {
        let id = Identity::new();
        let id2 = Identity::new();
        let minter_addr = Address::normal_from_pkey(*id.pkey());
        let minter_norm_address = NormalAddress::from_pkey(*id.pkey());
        let new_minter_addr = Address::normal_from_pkey(*id2.pkey());

        let mut db = test_helpers::init_tempdb();
        let mut root = Hash::NULL_RLP;
        let mut trie = TrieDBMut::<BlakeDbHasher, Codec>::new(&mut db, &mut root);

        let asset_hash = crypto::hash_slice(b"Test currency 1");
        let fee_hash = crypto::hash_slice(b"Test currency 2");

        // Manually initialize minter balance
        test_helpers::init_balance(&mut trie, minter_addr.clone(), fee_hash, b"100.0");

        let fee = Balance::from_bytes(b"10.0").unwrap();

        let mut tx = ChangeMinter {
            minter: minter_norm_address.clone(),
            new_minter: new_minter_addr.clone(),
            asset_hash: asset_hash,
            fee_hash: fee_hash,
            fee: fee.clone(),
            nonce: 1,
            signature: None,
            hash: None,
        };

        tx.sign(id.skey().clone());
        tx.compute_hash();

        assert!(tx.validate(&trie));
    }

    #[test]
    fn validate_cannot_pay_fee() {
        let id = Identity::new();
        let id2 = Identity::new();
        let minter_addr = Address::normal_from_pkey(*id.pkey());
        let minter_norm_address = NormalAddress::from_pkey(*id.pkey());
        let new_minter_addr = Address::normal_from_pkey(*id2.pkey());

        let mut db = test_helpers::init_tempdb();
        let mut root = Hash::NULL_RLP;
        let mut trie = TrieDBMut::<BlakeDbHasher, Codec>::new(&mut db, &mut root);

        let asset_hash = crypto::hash_slice(b"Test currency 1");
        let fee_hash = crypto::hash_slice(b"Test currency 2");

        // Manually initialize minter balance
        test_helpers::init_balance(&mut trie, minter_addr.clone(), fee_hash, b"100.0");

        let fee = Balance::from_bytes(b"1000.0").unwrap();

        let mut tx = ChangeMinter {
            minter: minter_norm_address.clone(),
            new_minter: new_minter_addr.clone(),
            asset_hash: asset_hash,
            fee_hash: fee_hash,
            fee: fee.clone(),
            nonce: 1,
            signature: None,
            hash: None,
        };

        tx.sign(id.skey().clone());
        tx.compute_hash();

        assert!(!tx.validate(&trie));
    }

    #[test]
    fn validate_no_minter() {
        let id = Identity::new();
        let id2 = Identity::new();
        let minter_norm_address = NormalAddress::from_pkey(*id.pkey());
        let new_minter_addr = Address::normal_from_pkey(*id2.pkey());

        let mut db = test_helpers::init_tempdb();
        let mut root = Hash::NULL_RLP;
        let trie = TrieDBMut::<BlakeDbHasher, Codec>::new(&mut db, &mut root);

        let asset_hash = crypto::hash_slice(b"Test currency 1");
        let fee_hash = crypto::hash_slice(b"Test currency 2");

        let fee = Balance::from_bytes(b"10.0").unwrap();

        let mut tx = ChangeMinter {
            minter: minter_norm_address.clone(),
            new_minter: new_minter_addr.clone(),
            asset_hash: asset_hash,
            fee_hash: fee_hash,
            fee: fee.clone(),
            nonce: 1,
            signature: None,
            hash: None,
        };

        tx.sign(id.skey().clone());
        tx.compute_hash();

        assert!(!tx.validate(&trie));
    }

    #[test]
    fn apply_it_changes_minter() {
        // Create Mintable first
        let id = Identity::new();
        let id2 = Identity::new();
        let creator_addr = Address::normal_from_pkey(*id.pkey());
        let creator_norm_address = NormalAddress::from_pkey(*id.pkey());
        let minter_addr = Address::normal_from_pkey(*id2.pkey());
        let asset_hash = crypto::hash_slice(b"Test currency 1");
        let fee_hash = crypto::hash_slice(b"Test currency 2");

        let mut db = test_helpers::init_tempdb();
        let mut root = Hash::NULL_RLP;
        let mut trie = TrieDBMut::<BlakeDbHasher, Codec>::new(&mut db, &mut root);

        // Manually initialize creator balance
        test_helpers::init_balance(&mut trie, creator_addr.clone(), fee_hash, b"100.0");

        let fee = Balance::from_bytes(b"10.0").unwrap();

        let mut tx = CreateMintable {
            creator: creator_norm_address.clone(),
            receiver: creator_addr.clone(),
            minter_address: minter_addr.clone(),
            coin_supply: 100,
            max_supply: 200,
            precision: 18,
            fee: fee.clone(),
            asset_hash: asset_hash,
            fee_hash: fee_hash,
            nonce: 1,
            signature: None,
            hash: None,
        };

        tx.sign(id.skey().clone());
        tx.compute_hash();

        // Apply transaction
        tx.apply(&mut trie);

        // Commit changes
        trie.commit();

        let bin_asset_hash = asset_hash.to_vec();
        let hex_asset_hash = hex::encode(&bin_asset_hash);
        let asset_hash_minter_key = format!("{}.m", hex_asset_hash);
        let asset_hash_minter_key = asset_hash_minter_key.as_bytes();

        // Check minter address
        assert_eq!(
            &trie.get(&asset_hash_minter_key).unwrap().unwrap(),
            &minter_addr.to_bytes()
        );

        let id3 = Identity::new();
        let new_minter_addr = Address::normal_from_pkey(*id3.pkey());

        assert_ne!(
            &trie.get(&asset_hash_minter_key).unwrap().unwrap(),
            &new_minter_addr.to_bytes()
        );

        let mut tx = ChangeMinter {
            minter: creator_norm_address.clone(),
            new_minter: new_minter_addr.clone(),
            asset_hash: asset_hash,
            fee_hash: fee_hash,
            fee: fee.clone(),
            nonce: 2,
            signature: None,
            hash: None,
        };

        tx.sign(id.skey().clone());
        tx.compute_hash();

        // Apply transaction
        tx.apply(&mut trie);

        // Commit changes
        trie.commit();

        // Check minter address
        assert_ne!(
            &trie.get(&asset_hash_minter_key).unwrap().unwrap(),
            &minter_addr.to_bytes()
        );

        assert_eq!(
            &trie.get(&asset_hash_minter_key).unwrap().unwrap(),
            &new_minter_addr.to_bytes()
        );
    }

    quickcheck! {
        fn serialize_deserialize(tx: ChangeMinter) -> bool {
            tx == ChangeMinter::from_bytes(&ChangeMinter::to_bytes(&tx).unwrap()).unwrap()
        }

        fn verify_hash(tx: ChangeMinter) -> bool {
            let mut tx = tx;

            for _ in 0..3 {
                tx.compute_hash();
            }

            tx.verify_hash()
        }

        fn verify_signature(
            new_minter: Address,
            fee: Balance,
            asset_hash: Hash,
            fee_hash: Hash
        ) -> bool {
            let id = Identity::new();

            let mut tx = ChangeMinter {
                minter: NormalAddress::from_pkey(*id.pkey()),
                new_minter: new_minter,
                fee: fee,
                asset_hash: asset_hash,
                fee_hash: fee_hash,
                nonce: 1,
                signature: None,
                hash: None
            };

            tx.sign(id.skey().clone());
            tx.verify_sig()
        }
    }
}
