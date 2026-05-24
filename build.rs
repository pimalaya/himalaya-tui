// This file is part of Himalaya TUI, a TUI to manage emails.
//
// Copyright (C) 2025-2026  soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use pimalaya_cli::build::{features_env, target_envs};

fn main() {
    features_env(include_str!("./Cargo.toml"));
    target_envs();

    println!("cargo::rustc-env=GIT_DESCRIBE=v0.1.0");
    println!("cargo::rustc-env=GIT_REV=dev");
}
