// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use uuid::Uuid;
use crate::config::{Account, AccountType};

pub fn create_offline_account(username: String) -> Account {
    Account {
        id: Uuid::new_v4().to_string(),
        username,
        account_type: AccountType::Offline,
        access_token: None,
        refresh_token: None,
        xbox_uid: None,
    }
}
