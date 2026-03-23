// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;

#[derive(Default)]
pub struct Cache {
    values: HashMap<String, f64>,
}

impl Cache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<f64> {
        self.values.get(key).copied()
    }

    pub fn set(&mut self, key: &str, value: f64) {
        self.values.insert(key.to_string(), value);
    }
}
