// Copyright 2019 KTH Royal Institute of Technology. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use failure::Fail;

#[derive(Debug, Fail)]
pub enum ManagerError {
    #[fail(display = "Error while handling API request: {}", msg)]
    ApiRequestError { msg: String },
    #[fail(display = "Error while parsing JSON: {}", msg)]
    JsonParseError { msg: String },
}
