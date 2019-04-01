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

//! Foundation bits exposing the Compute API.

use std::collections::HashMap;
use std::fmt::Debug;

use reqwest::RequestBuilder;
use serde::Serialize;
use serde_json;

use super::super::common::protocol::Ref;
use super::super::common::{self, ApiVersion};
use super::super::session::{RequestBuilderExt, ServiceType, Session};
use super::super::utils::{self, ResultExt};
use super::super::Result;
use super::protocol;

const API_VERSION_KEYPAIR_TYPE: ApiVersion = ApiVersion(2, 2);
const API_VERSION_SERVER_DESCRIPTION: ApiVersion = ApiVersion(2, 19);
const API_VERSION_KEYPAIR_PAGINATION: ApiVersion = ApiVersion(2, 35);
const API_VERSION_FLAVOR_DESCRIPTION: ApiVersion = ApiVersion(2, 55);
const API_VERSION_FLAVOR_EXTRA_SPECS: ApiVersion = ApiVersion(2, 61);

/// Service type of Compute API ComputeService.
#[derive(Copy, Clone, Debug)]
pub struct ComputeService;

impl ServiceType for ComputeService {
    fn catalog_type() -> &'static str {
        "compute"
    }

    fn major_version_supported(version: ApiVersion) -> bool {
        version.0 == 2
    }

    fn set_api_version_headers(
        request: RequestBuilder,
        version: ApiVersion,
    ) -> Result<RequestBuilder> {
        // TODO: new-style header support
        Ok(request.header("x-openstack-nova-api-version", version.to_string()))
    }
}

/// Pick the highest API version or None if neither is supported.
fn pick_compute_api_version(
    session: &Session,
    versions: &[ApiVersion],
) -> Result<Option<ApiVersion>> {
    let info = session.get_service_info_ref::<ComputeService>()?;
    Ok(versions
        .iter()
        .filter(|item| info.supports_api_version(**item))
        .max()
        .cloned())
}

fn flavor_api_version(session: &Session) -> Result<Option<ApiVersion>> {
    pick_compute_api_version(
        session,
        &[
            API_VERSION_FLAVOR_DESCRIPTION,
            API_VERSION_FLAVOR_EXTRA_SPECS,
        ],
    )
}

fn supports_compute_api_version(session: &Session, version: ApiVersion) -> Result<bool> {
    let info = session.get_service_info_ref::<ComputeService>()?;
    Ok(info.supports_api_version(version))
}

/// Create a key pair.
pub fn create_keypair(
    session: &Session,
    request: protocol::KeyPairCreate,
) -> Result<protocol::KeyPair> {
    let version = if request.key_type.is_some() {
        Some(API_VERSION_KEYPAIR_TYPE)
    } else {
        None
    };

    debug!("Creating a key pair with {:?}", request);
    let body = protocol::KeyPairCreateRoot { keypair: request };
    let keypair = session
        .post::<ComputeService>(&["os-keypairs"], version)?
        .json(&body)
        .receive_json::<protocol::KeyPairRoot>()?
        .keypair;
    debug!("Created key pair {:?}", keypair);
    Ok(keypair)
}

/// Create a server.
pub fn create_server(session: &Session, request: protocol::ServerCreate) -> Result<Ref> {
    debug!("Creating a server with {:?}", request);
    let body = protocol::ServerCreateRoot { server: request };
    let server = session
        .post::<ComputeService>(&["servers"], None)?
        .json(&body)
        .receive_json::<protocol::CreatedServerRoot>()?
        .server;
    trace!("Requested creation of server {:?}", server);
    Ok(server)
}

/// Delete a key pair.
pub fn delete_keypair<S: AsRef<str>>(session: &Session, name: S) -> Result<()> {
    debug!("Deleting key pair {}", name.as_ref());
    session
        .delete::<ComputeService>(&["os-keypairs", name.as_ref()], None)?
        .commit()?;
    debug!("Key pair {} was deleted", name.as_ref());
    Ok(())
}

/// Delete a server.
pub fn delete_server<S: AsRef<str>>(session: &Session, id: S) -> Result<()> {
    trace!("Deleting server {}", id.as_ref());
    session
        .delete::<ComputeService>(&["servers", id.as_ref()], None)?
        .commit()?;
    debug!("Successfully requested deletion of server {}", id.as_ref());
    Ok(())
}

/// Get a flavor by its ID.
pub fn get_extra_specs_by_flavor_id<S: AsRef<str>>(
    session: &Session,
    id: S,
) -> Result<HashMap<String, String>> {
    trace!("Get compute extra specs by ID {}", id.as_ref());
    let extra_specs = session
        .get::<ComputeService>(&["flavors", id.as_ref(), "os-extra_specs"], None)?
        .receive_json::<protocol::ExtraSpecsRoot>()?
        .extra_specs;
    trace!("Received {:?}", extra_specs);
    Ok(extra_specs)
}

/// Get a flavor.
pub fn get_flavor<S: AsRef<str>>(session: &Session, id_or_name: S) -> Result<protocol::Flavor> {
    let s = id_or_name.as_ref();
    get_flavor_by_id(session, s).if_not_found_then(|| get_flavor_by_name(session, s))
}

/// Get a flavor by its ID.
pub fn get_flavor_by_id<S: AsRef<str>>(session: &Session, id: S) -> Result<protocol::Flavor> {
    trace!("Get compute flavor by ID {}", id.as_ref());
    let version = flavor_api_version(session)?;
    let flavor = session
        .get::<ComputeService>(&["flavors", id.as_ref()], version)?
        .receive_json::<protocol::FlavorRoot>()?
        .flavor;
    trace!("Received {:?}", flavor);
    Ok(flavor)
}

/// Get a flavor by its name.
pub fn get_flavor_by_name<S: AsRef<str>>(session: &Session, name: S) -> Result<protocol::Flavor> {
    trace!("Get compute flavor by name {}", name.as_ref());
    let items = session
        .get::<ComputeService>(&["flavors"], None)?
        .receive_json::<protocol::FlavorsRoot>()?
        .flavors
        .into_iter()
        .filter(|item| item.name == name.as_ref());
    utils::one(
        items,
        "Flavor with given name or ID not found",
        "Too many flavors found with given name",
    )
    .and_then(|item| get_flavor_by_id(session, item.id))
}

/// Get a key pair by its name.
pub fn get_keypair<S: AsRef<str>>(session: &Session, name: S) -> Result<protocol::KeyPair> {
    trace!("Get compute key pair by name {}", name.as_ref());
    let ver = pick_compute_api_version(session, &[API_VERSION_KEYPAIR_TYPE])?;
    let keypair = session
        .get::<ComputeService>(&["os-keypairs", name.as_ref()], ver)?
        .receive_json::<protocol::KeyPairRoot>()?
        .keypair;
    trace!("Received {:?}", keypair);
    Ok(keypair)
}

/// Get a server.
pub fn get_server<S: AsRef<str>>(session: &Session, id_or_name: S) -> Result<protocol::Server> {
    let s = id_or_name.as_ref();
    get_server_by_id(session, s).if_not_found_then(|| get_server_by_name(session, s))
}

/// Get a server by its ID.
pub fn get_server_by_id<S: AsRef<str>>(session: &Session, id: S) -> Result<protocol::Server> {
    trace!("Get compute server with ID {}", id.as_ref());
    let version = pick_compute_api_version(session, &[API_VERSION_SERVER_DESCRIPTION])?;
    let server = session
        .get::<ComputeService>(&["servers", id.as_ref()], version)?
        .receive_json::<protocol::ServerRoot>()?
        .server;
    trace!("Received {:?}", server);
    Ok(server)
}

/// Get a server by its name.
pub fn get_server_by_name<S: AsRef<str>>(session: &Session, name: S) -> Result<protocol::Server> {
    trace!("Get compute server with name {}", name.as_ref());
    let items = session
        .get::<ComputeService>(&["servers"], None)?
        .query(&[("name", name.as_ref())])
        .receive_json::<protocol::ServersRoot>()?
        .servers
        .into_iter()
        .filter(|item| item.name == name.as_ref());
    utils::one(
        items,
        "Server with given name or ID not found",
        "Too many servers found with given name",
    )
    .and_then(|item| get_server_by_id(session, item.id))
}

/// List flavors.
pub fn list_flavors<Q: Serialize + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<common::protocol::IdAndName>> {
    trace!("Listing compute flavors with {:?}", query);
    let result = session
        .get::<ComputeService>(&["flavors"], None)?
        .query(query)
        .receive_json::<protocol::FlavorsRoot>()?
        .flavors;
    trace!("Received flavors: {:?}", result);
    Ok(result)
}

/// List flavors with details.
pub fn list_flavors_detail<Q: Serialize + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<protocol::Flavor>> {
    trace!("Listing compute flavors with {:?}", query);
    let version = pick_compute_api_version(session, &[API_VERSION_FLAVOR_EXTRA_SPECS])?;
    let result = session
        .get::<ComputeService>(&["flavors", "detail"], version)?
        .query(query)
        .receive_json::<protocol::FlavorsDetailRoot>()?
        .flavors;
    trace!("Received flavors: {:?}", result);
    Ok(result)
}

/// List key pairs.
pub fn list_keypairs<Q: Serialize + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<protocol::KeyPair>> {
    trace!("Listing compute key pairs with {:?}", query);
    let ver = pick_compute_api_version(
        session,
        &[API_VERSION_KEYPAIR_TYPE, API_VERSION_KEYPAIR_PAGINATION],
    )?;
    let result = session
        .get::<ComputeService>(&["os-keypairs"], ver)?
        .query(query)
        .receive_json::<protocol::KeyPairsRoot>()?
        .keypairs
        .into_iter()
        .map(|item| item.keypair)
        .collect::<Vec<_>>();
    trace!("Received key pairs: {:?}", result);
    Ok(result)
}

/// List servers.
pub fn list_servers<Q: Serialize + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<common::protocol::IdAndName>> {
    trace!("Listing compute servers with {:?}", query);
    let result = session
        .get::<ComputeService>(&["servers"], None)?
        .query(query)
        .receive_json::<protocol::ServersRoot>()?
        .servers;
    trace!("Received servers: {:?}", result);
    Ok(result)
}

/// List servers with details.
pub fn list_servers_detail<Q: Serialize + Debug>(
    session: &Session,
    query: &Q,
) -> Result<Vec<protocol::Server>> {
    trace!("Listing compute servers with {:?}", query);
    let version = pick_compute_api_version(session, &[API_VERSION_SERVER_DESCRIPTION])?;
    let result = session
        .get::<ComputeService>(&["servers", "detail"], version)?
        .query(query)
        .receive_json::<protocol::ServersDetailRoot>()?
        .servers;
    trace!("Received servers: {:?}", result);
    Ok(result)
}

/// Run an action while providing some arguments.
pub fn server_action_with_args<S1, S2, Q>(
    session: &Session,
    id: S1,
    action: S2,
    args: Q,
) -> Result<()>
where
    S1: AsRef<str>,
    S2: AsRef<str>,
    Q: Serialize + Debug,
{
    trace!(
        "Running {} on server {} with args {:?}",
        action.as_ref(),
        id.as_ref(),
        args
    );
    let mut body = HashMap::new();
    let _ = body.insert(action.as_ref(), args);
    session
        .post::<ComputeService>(&["servers", id.as_ref(), "action"], None)?
        .json(&body)
        .commit()?;
    debug!(
        "Successfully ran {} on server {}",
        action.as_ref(),
        id.as_ref()
    );
    Ok(())
}

/// Run an action on the server.
pub fn server_simple_action<S1, S2>(session: &Session, id: S1, action: S2) -> Result<()>
where
    S1: AsRef<str>,
    S2: AsRef<str>,
{
    server_action_with_args(session, id, action, serde_json::Value::Null)
}

/// Whether key pair pagination is supported.
pub fn supports_keypair_pagination(session: &Session) -> Result<bool> {
    supports_compute_api_version(session, API_VERSION_KEYPAIR_PAGINATION)
}
