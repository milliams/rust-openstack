// Copyright 2018 Dmitry Tantsur <divius.inside@gmail.com>
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

//! Types and traits shared by all API parts.

mod apiversion;
pub(crate) mod protocol;
mod resourceiterator;
mod types;
mod waiter;

pub use self::apiversion::ApiVersion;
pub use self::resourceiterator::ResourceIterator;
pub use self::types::{FlavorRef, ImageRef, KeyPairRef, ListResources,
                      NetworkRef, PortRef, ProjectRef, Refresh, ResourceId,
                      SubnetRef, UserRef};
pub use self::waiter::DeletionWaiter;
