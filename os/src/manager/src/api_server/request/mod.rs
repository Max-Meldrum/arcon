// Copyright 2018 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// Modifications copyright (C) 2019 KTH Royal Institute of Technology
// SPDX-License-Identifier: Apache-2.0

use crate::error::ManagerError;
use micro_http::{Body, Method, Request, Response, StatusCode, Version};

pub enum ApiRequest {
    GetSpec,
    GetJobs,
    GetResourcePool,
}

impl ApiRequest {
    pub fn parse_request(request: &Request) -> Result<Self, ManagerError> {
        let uri = request.uri().get_abs_path().to_string();
        let path_tokens: Vec<&str> = uri[1..].split_terminator('/').collect();
        let path = if path_tokens.is_empty() {
            ""
        } else {
            path_tokens[0]
        };

        match (request.method(), path, request.body.as_ref()) {
            (Method::Get, "", None) => Ok(ApiRequest::GetSpec),
            (Method::Get, "spec", None) => Ok(ApiRequest::GetSpec),
            (Method::Get, "jobs", None) => Ok(ApiRequest::GetJobs),
            (Method::Get, "resources", None) => Ok(ApiRequest::GetResourcePool),
            (Method::Get, &_, Some(_)) => method_to_error(Method::Get),
            (Method::Put, _, _) => Ok(ApiRequest::GetSpec),
            (Method::Patch, _, _) => Ok(ApiRequest::GetSpec),
            (_, unknown_uri, _) => {
                let msg = format!("Unknown URI {}", unknown_uri);
                Err(ManagerError::ApiRequestError { msg: msg })
            }
        }
    }
}

pub fn method_to_error(method: Method) -> Result<ApiRequest, ManagerError> {
    match method {
        Method::Get => Err(ManagerError::ApiRequestError {
            msg: "GET request cannot have body".to_string(),
        }),
        Method::Put => Err(ManagerError::ApiRequestError {
            msg: "Empty PUT request".to_string(),
        }),
        Method::Patch => Err(ManagerError::ApiRequestError {
            msg: "Empty PATCH request".to_string(),
        }),
    }
}
