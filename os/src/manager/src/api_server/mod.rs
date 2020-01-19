// Copyright 2018 Amazon.com, Inc. or its affiliates. All Rights Reserved.
// Modifications copyright (C) 2019 KTH Royal Institute of Technology
// SPDX-License-Identifier: Apache-2.0

mod request;

use crate::api_server::request::ApiRequest;
use crate::error::ManagerError;
use crate::Manager;
use micro_http::{
    Body, HttpServer, Method, Request, RequestError, Response, ServerError, ServerRequest,
    ServerResponse, StatusCode, Version,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct ApiServer {
    manager: Arc<Mutex<Manager>>,
}

impl ApiServer {
    pub fn new(manager: Arc<Mutex<Manager>>) -> ApiServer {
        ApiServer { manager }
    }

    pub fn run(&mut self, path: PathBuf) -> Result<(), ManagerError> {
        let mut server = HttpServer::new(path).unwrap();
        server.start_server().unwrap();
        loop {
            match server.requests() {
                Ok(requests) => {
                    for server_request in requests {
                        let _ = server
                            .respond(
                                // Use `self.handle_request()` as the processing callback.
                                server_request.process(|request| self.handle_request(request)),
                            )
                            .or_else(|e: ServerError| {
                                error!("API Server encountered an error on response: {}", e);
                                Ok(())
                            })?;
                    }
                }
                Err(e) => error!("API Server error: {}", e),
            }
        }
    }

    fn handle_request(&self, request: &Request) -> Response {
        match ApiRequest::parse_request(request) {
            Ok(ApiRequest::GetSpec) => self.get_spec(),
            Ok(ApiRequest::GetJobs) => self.get_jobs(),
            Ok(ApiRequest::GetResourcePool) => self.get_resource_pool(),
            Err(err) => ApiServer::json_response(StatusCode::BadRequest, err.to_string()),
        }
    }

    fn get_spec(&self) -> Response {
        let manager = self.manager.lock().unwrap();
        let spec = (*manager).get_spec_json().unwrap();
        ApiServer::json_response(StatusCode::OK, spec)
    }

    fn get_jobs(&self) -> Response {
        let manager = self.manager.lock().unwrap();
        let spec = (*manager).get_jobs_json().unwrap();
        ApiServer::json_response(StatusCode::OK, spec)
    }

    fn get_resource_pool(&self) -> Response {
        let manager = self.manager.lock().unwrap();
        let spec = (*manager).get_resource_pool_json().unwrap();
        ApiServer::json_response(StatusCode::OK, spec)
    }

    /// An HTTP response which also includes a body.
    pub fn json_response<T: Into<String>>(status: StatusCode, body: T) -> Response {
        let mut response = Response::new(Version::Http11, status);
        response.set_body(Body::new(body.into()));
        response
    }
}
