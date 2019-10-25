/* Copyright (c) Fortanix, Inc.
 *
 * Licensed under the GNU General Public License, version 2 <LICENSE-GPL or
 * https://www.gnu.org/licenses/gpl-2.0.html> or the Apache License, Version
 * 2.0 <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0>, at your
 * option. This file may not be copied, modified, or distributed except
 * according to those terms. */

use bindgen;

use std::fs::File;
use std::io::Write;

use crate::headers;
/*
#[derive(Debug)]
struct StderrLogger;

impl bindgen::Logger for StderrLogger {
    fn error(&self, msg: &str) {
        let _ = writeln!(stderr(), "Bindgen ERROR: {}", msg);
    }
    fn warn(&self, msg: &str) {
        let _ = writeln!(stderr(), "Bindgen WARNING: {}", msg);
    }
}
*/

#[derive(Debug)]
struct ParseCallback;
use bindgen::callbacks::IntKind;

impl bindgen::callbacks::ParseCallbacks for ParseCallback {
    fn int_macro(&self, name: &str, _value: i64) -> Option<IntKind> {
        if name.contains("MBEDTLS_") {
            Some(IntKind::I32)
        } else {
            None
        }
    }
    fn item_name(&self, original_item_name: &str) -> Option<String> {
        if original_item_name.starts_with("mbedtls_time_t") {
            Some(original_item_name.to_string())
        } else if original_item_name.starts_with("cipher_mode_t_MBEDTLS_") {
            Some(
                original_item_name
                    .trim_start_matches("cipher_mode_t_MBEDTLS_")
                    .to_string(),
            )
        } else if original_item_name.starts_with("mbedtls_") {
            Some(
                original_item_name
                    .trim_start_matches("mbedtls_")
                    .to_string(),
            )
        } else if original_item_name.starts_with("MBEDTLS_") {
            Some(
                original_item_name
                    .trim_start_matches("MBEDTLS_")
                    .to_string(),
            )
        } else {
            None
        }
    }
}

impl super::BuildConfig {
    pub fn bindgen(&self) {
        let header = self.out_dir.join("bindgen-input.h");
        File::create(&header)
            .and_then(|mut f| {
                Ok(for h in headers::enabled_ordered() {
                    writeln!(f, "#include <mbedtls/{}>", h)?;
                })
            })
            .expect("bindgen-input.h I/O error");

        let include = self.mbedtls_src.join("include");

        //let logger = StderrLogger;
        let bindgen =
            bindgen::Builder::default().header(header.into_os_string().into_string().unwrap());
        let bindings = bindgen
            .clang_arg("-Dmbedtls_t_udbl=mbedtls_t_udbl;") // bindgen can't handle unused uint128
            .clang_arg(format!(
                "-DMBEDTLS_CONFIG_FILE=<{}>",
                self.config_h.to_str().expect("config.h UTF-8 error")
            ))
            .clang_arg(format!(
                "-I{}",
                include.to_str().expect("include/ UTF-8 error")
            ))
            //.match_pat(include.to_str().expect("include/ UTF-8 error"))
            //.match_pat(self.config_h.to_str().expect("config.h UTF-8 error"))
            .use_core()
            .derive_debug(false) // buggy :(
            .parse_callbacks(Box::new(ParseCallback))
            .ctypes_prefix("crate::types::raw_types")
            .blacklist_function("strtold")
            .blacklist_function("qecvt_r")
            .blacklist_function("qecvt")
            .blacklist_function("qfcvt_r")
            .blacklist_function("qgcvt")
            .blacklist_function("qfcvt")
            .opaque_type("std::*")
            .generate_comments(false)
            .generate()
            .expect("bindgen error");

        let bindings_rs = self.out_dir.join("bindings.rs");
        File::create(&bindings_rs)
            .and_then(|mut f| {
                f.write_all(b"#![allow(nonstandard_style)]\n#![allow(unused_imports)]\n")?;
                bindings.write(Box::new(&mut f))?;
                f.write_all(b"use crate::types::*;\n") // for FILE, time_t, etc.
            })
            .expect("bindings.rs I/O error");
        use std::process::Command;

        Command::new("sed")
            .args(&[
                "-i",
                "-e",
                "s# [a-zA-Z_]*_MBEDTLS_# #g;",
                bindings_rs.as_os_str().to_string_lossy().as_ref(),
            ])
            .status()
            .unwrap();

        let mod_bindings = self.out_dir.join("mod-bindings.rs");
        File::create(&mod_bindings)
            .and_then(|mut f| f.write_all(b"mod bindings;\n"))
            .expect("mod-bindings.rs I/O error");
    }
}
