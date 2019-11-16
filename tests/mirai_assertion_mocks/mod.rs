/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

#[macro_export]
macro_rules! debug_checked_precondition {
    ($condition:expr, $($arg:tt)*) => ( debug_assert!($condition, $($arg)*); );
}

#[macro_export]
macro_rules! debug_checked_postcondition {
    ($condition:expr, $($arg:tt)*) => ( debug_assert!($condition, $($arg)*); );
}

#[macro_export]
macro_rules! checked_precondition {
    ($condition:expr, $($arg:tt)*) => ( assert!($condition, $($arg)*); );
}

#[macro_export]
macro_rules! checked_postcondition {
    ($condition:expr, $($arg:tt)*) => ( assert!($condition, $($arg)*); );
}
