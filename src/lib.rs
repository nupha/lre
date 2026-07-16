// SPDX-License-Identifier: MIT
// Copyright (c) 2025 John Ray <996351336@qq.com>

//! Rust bindings for the libregexp regular expression library.
//!
//! This crate provides safe Rust bindings to the libregexp C library,
//! which is a lightweight regular expression engine from QuickJS.

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::missing_safety_doc)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

mod error;
mod ffi;
mod regex;
mod safe;

pub use {error::RegexError, regex::Regex, safe::RegexFlags};
