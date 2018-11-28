/*
  Copyright 2018 The Purple Library Authors
  This file is part of the Purple Library.

  The Purple Library is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  The Purple Library is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with the Purple Library. If not, see <http://www.gnu.org/licenses/>.
*/

#![feature(extern_prelude)]

#[macro_use] extern crate serde_derive;
#[macro_use] extern crate quickcheck;

extern crate crypto;
extern crate rlp;
extern crate byteorder;
extern crate regex;
extern crate rand;
extern crate rust_decimal;

mod addresses;
mod balance;
mod signature;
mod multi_sig;
mod shares;
mod share_map;

pub use addresses::normal::*;
pub use addresses::multi_sig::*;
pub use addresses::shareholders::*;
pub use addresses::*;
pub use balance::*;
pub use signature::*;
pub use multi_sig::*;
pub use shares::*;
pub use share_map::*;