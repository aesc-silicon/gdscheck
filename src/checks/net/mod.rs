// SPDX-FileCopyrightText: 2026 aesc silicon
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Net: what a rule can only ask once the layout's nets are known - the antenna ratio
//! of a gate's net level by level up the stack, and how many nets lie under one marker.
//! Both read the connect graph the PDK declares (see `Connectivity`), and a deck that
//! names one runs net extraction.  The spacing and area families read nets too, as
//! gates on their own measurements: `net: same`, `net: different`, `net: connected`.

pub mod antenna;
pub mod nets_under;
