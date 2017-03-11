// Copyright 2017 Dmitry Tantsur <divius.inside@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Generic API bits for implementing new services.

use std::marker::PhantomData;

use hyper::{Get, Url};
use serde::Deserialize;
use serde_json;

use super::{ApiResult, ApiVersion, Session};
use super::auth::Method as AuthMethod;
use super::utils;
pub use super::utils::IntoId;


/// Information about API endpoint.
#[derive(Clone, Debug)]
pub struct ServiceInfo {
    /// Root endpoint.
    pub root_url: Url,
    /// Current API version (if supported).
    pub current_version: Option<ApiVersion>,
    /// Minimum API version (if supported).
    pub minimum_version: Option<ApiVersion>
}

/// Trait representing a service type.
pub trait ServiceType {
    /// Service type to pass to the catalog.
    fn catalog_type() -> &'static str;

    /// Get basic service information.
    fn service_info<Auth: AuthMethod>(endpoint: Url, session: &Session<Auth>)
        -> ApiResult<ServiceInfo>;
}

/// Low-level API calls.
#[derive(Debug)]
pub struct ServiceApi<'a, Auth: AuthMethod + 'a, Service> {
    session: &'a Session<Auth>,
    service_type: PhantomData<Service>,
    endpoint_interface: Option<String>,
    cached_info: utils::ValueCache<ServiceInfo>
}


impl<'a, Auth: AuthMethod + 'a, S: ServiceType> ServiceApi<'a, Auth, S> {
    /// Create a new API instance using the given session.
    pub fn new(session: &'a Session<Auth>) -> ServiceApi<'a, Auth, S> {
        ServiceApi {
            session: session,
            service_type: PhantomData,
            endpoint_interface: None,
            cached_info: utils::ValueCache::new(None)
        }
    }

    /// Create a new API instance using the given session.
    pub fn new_with_endpoint<S1>(session: &'a Session<Auth>,
                                 endpoint_interface: S1)
            -> ServiceApi<'a, Auth, S> where S1: Into<String> {
        ServiceApi {
            session: session,
            service_type: PhantomData,
            endpoint_interface: Some(endpoint_interface.into()),
            cached_info: utils::ValueCache::new(None)
        }
    }

    /// Get the root endpoint of the service.
    ///
    /// The resulting endpoint is cached on the current ServiceApi object.
    pub fn get_root_endpoint(&self) -> ApiResult<Url> {
        try!(self.cached_info.ensure_value(|| {
            let ep = try!(match self.endpoint_interface {
                Some(ref s) => self.session.get_endpoint(S::catalog_type(),
                                                         s.clone()),
                None => self.session.get_default_endpoint(S::catalog_type())
            });

            S::service_info(ep, &self.session)
        }));

        let info = self.cached_info.get().unwrap();
        Ok(info.root_url.clone())
    }

    /// Get an endpoint with version suffix and given path appended.
    pub fn get_endpoint(&self, path: &str) -> ApiResult<Url> {
        let endpoint = try!(self.get_root_endpoint());
        Ok(utils::url::join(endpoint, path))
    }

    /// List entities.
    pub fn list<R: Deserialize>(&self, path: &str) -> ApiResult<R> {
        // TODO: filtering
        let url = try!(self.get_endpoint(path));
        debug!("Listing entities from {}", url);
        let resp = try!(self.session.request(Get, url).send());
        let root = try!(serde_json::from_reader(resp));
        Ok(root)
    }

    /// Get one entity.
    pub fn get<R: Deserialize, Id: IntoId>(&self, path: &str, id: Id)
            -> ApiResult<R> {
        let url = try!(self.get_endpoint(&path));
        let url_with_id = utils::url::join(url, &id.into_id());
        debug!("Get one entity from {}", url_with_id);
        let resp = try!(self.session.request(Get, url_with_id).send());
        let root = try!(serde_json::from_reader(resp));
        Ok(root)
    }
}
