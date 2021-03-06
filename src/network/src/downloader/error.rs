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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DownloaderErr {
    /// The downloader buffer is full
    Full,

    /// We already have info about the piece
    AlreadyHaveInfo,

    /// The provided info is invalid
    InvalidInfo,

    /// Checksum validation failed
    InvalidChecksum,

    /// Received a duplicate checksum
    DuplicateChecksum,

    /// The provided size is invalid
    InvalidSize,

    /// Could not schedule block download as the header is invalid
    InvalidBlockHeader,

    /// Could not find object with the given query
    NotFound,

    /// We already have the sub-piece data
    AlreadyHaveData,

    /// The download for the given object is already scheduled
    AlreadyHaveDownload,
}
