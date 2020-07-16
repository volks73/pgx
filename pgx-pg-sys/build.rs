// Copyright 2020 ZomboDB, LLC <zombodb@gmail.com>. All rights reserved. Use of this source code is
// governed by the MIT license that can be found in the LICENSE file.

extern crate build_deps;

use bindgen::callbacks::MacroParsingBehavior;
use pgx_utils::{exit_with_error, get_pgx_config_path, handle_result, prefix_path, run_pg_config};
use quote::quote;
use rayon::prelude::*;
use serde_derive::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::str::FromStr;
use std::sync::Mutex;
use syn::export::{ToTokens, TokenStream2};
use syn::Item;

#[derive(Debug)]
struct IgnoredMacros(HashSet<String>);

impl IgnoredMacros {
    fn default() -> Self {
        // these cause duplicate definition problems on linux
        // see: https://github.com/rust-lang/rust-bindgen/issues/687
        IgnoredMacros(
            vec![
                "FP_INFINITE".into(),
                "FP_NAN".into(),
                "FP_NORMAL".into(),
                "FP_SUBNORMAL".into(),
                "FP_ZERO".into(),
                "IPPORT_RESERVED".into(),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl bindgen::callbacks::ParseCallbacks for IgnoredMacros {
    fn will_parse_macro(&self, name: &str) -> MacroParsingBehavior {
        if self.0.contains(name) {
            bindgen::callbacks::MacroParsingBehavior::Ignore
        } else {
            bindgen::callbacks::MacroParsingBehavior::Default
        }
    }
}

#[derive(Debug, Deserialize)]
struct PgConfigPaths {
    pg10: String,
    pg11: String,
    pg12: String,
}

#[derive(Debug, Deserialize)]
struct Configs {
    configs: PgConfigPaths,
}

fn load_pgx_config() -> Configs {
    let path = get_pgx_config_path();

    if !path.exists() {
        // TODO:  do this automatically if an environment variable is set?
        //        I think we want/need that ability
        exit_with_error!(
            "{} not found.  Have you run `{}` yet?",
            path.display(),
            "cargo pgx init".bold().yellow()
        )
    }

    handle_result!(
        "config.toml invalid",
        toml::from_str::<Configs>(handle_result!(
            "Unable to read config.toml",
            &std::fs::read_to_string(path)
        ))
    )
}

fn main() -> Result<(), std::io::Error> {
    let configs = load_pgx_config().configs;

    // dump our environment
    for (k, v) in std::env::vars() {
        eprintln!("{}={}", k, v);
    }

    build_deps::rerun_if_changed_paths(&get_pgx_config_path().display().to_string()).unwrap();
    build_deps::rerun_if_changed_paths("include/*").unwrap();
    build_deps::rerun_if_changed_paths("cshim/pgx-cshim.c").unwrap();
    build_deps::rerun_if_changed_paths("cshim/Makefile").unwrap();

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let shim_dir = PathBuf::from(format!("{}/cshim", manifest_dir.display()));

    eprintln!("manifest_dir={}", manifest_dir.display());
    eprintln!("shim_dir={}", shim_dir.display());

    let pg_configs = vec![
        (PathBuf::from_str(&configs.pg10).unwrap(), 10),
        (PathBuf::from_str(&configs.pg11).unwrap(), 11),
        (PathBuf::from_str(&configs.pg12).unwrap(), 12),
    ];
    let shim_mutex = Mutex::new(());

    pg_configs
        .into_par_iter()
        .for_each(|(pg_config, major_version)| {
            let include_h = PathBuf::from(format!(
                "{}/include/pg{}.h",
                manifest_dir.display(),
                major_version
            ));
            let bindings_rs = PathBuf::from(format!(
                "{}/src/pg{}_bindings.rs",
                manifest_dir.display(),
                major_version
            ));

            run_bindgen(&pg_config, major_version, &include_h, &bindings_rs);
            build_shim(&shim_dir, &shim_mutex, major_version, &pg_config);
        });

    generate_common_rs(manifest_dir);

    Ok(())
}

fn run_bindgen(
    pg_config: &PathBuf,
    major_version: u16,
    include_h: &PathBuf,
    bindings_rs: &PathBuf,
) {
    eprintln!(
        "Generating bindings for pg{} to {}",
        major_version,
        bindings_rs.display()
    );
    let includedir_server = run_pg_config(
        &Some(pg_config.display().to_string()),
        "--includedir-server",
    );
    let bindings = bindgen::Builder::default()
        .header(include_h.display().to_string())
        .clang_arg(&format!("-I{}", includedir_server))
        .parse_callbacks(Box::new(IgnoredMacros::default()))
        .blacklist_function("varsize_any") // pgx converts the VARSIZE_ANY macro, so we don't want to also have this function, which is in heaptuple.c
        .blacklist_function("query_tree_walker")
        .blacklist_function("expression_tree_walker")
        .blacklist_function("sigsetjmp")
        .blacklist_function("siglongjmp")
        .blacklist_function("pg_re_throw")
        .size_t_is_usize(true)
        .rustfmt_bindings(true)
        .derive_debug(true)
        .derive_copy(true) // necessary to avoid __BindgenUnionField usages -- I don't understand why?
        .derive_default(true)
        .derive_eq(false)
        .derive_partialeq(false)
        .derive_hash(false)
        .derive_ord(false)
        .derive_partialord(false)
        .layout_tests(false)
        .generate()
        .unwrap_or_else(|e| {
            panic!(
                "Unable to generate bindings for pg{}: {:?}",
                major_version, e
            )
        });

    let bindings = apply_pg_guard(bindings.to_string()).unwrap();
    std::fs::write(&bindings_rs, bindings).unwrap_or_else(|e| {
        panic!(
            "Unable to save bindings for pg{} to {}: {:?}",
            major_version,
            bindings_rs.display(),
            e
        )
    });
}

fn build_shim(shim_dir: &PathBuf, shim_mutex: &Mutex<()>, major_version: u16, pg_config: &PathBuf) {
    // build the shim under a lock b/c this can't be built concurrently
    let _lock = shim_mutex.lock().expect("couldn't obtain shim_mutex");

    // then build the shim for the version feature currently being built
    build_shim_for_version(&shim_dir, major_version, pg_config).expect("shim build failed");

    // and tell rustc to link to the library that was built for the feature we're currently building
    if std::env::var("CARGO_FEATURE_PG10").is_ok() {
        println!("cargo:rustc-link-search={}", shim_dir.display());
        println!("cargo:rustc-link-lib=static=pgx-cshim-10");
    } else if std::env::var("CARGO_FEATURE_PG11").is_ok() {
        println!("cargo:rustc-link-search={}", shim_dir.display());
        println!("cargo:rustc-link-lib=static=pgx-cshim-11");
    } else if std::env::var("CARGO_FEATURE_PG12").is_ok() {
        println!("cargo:rustc-link-search={}", shim_dir.display());
        println!("cargo:rustc-link-lib=static=pgx-cshim-12");
    }
}

fn build_shim_for_version(
    shim_dir: &PathBuf,
    major_version: u16,
    pg_config: &PathBuf,
) -> Result<(), std::io::Error> {
    let path_env = prefix_path(pg_config.parent().unwrap());

    eprintln!("PATH for build_shim={}", path_env);
    eprintln!("shim_dir={}", shim_dir.display());
    let rc = run_command(
        Command::new("make")
            .arg("clean")
            .arg(&format!("libpgx-cshim-{}.a", major_version))
            .env("PG_TARGET_VERSION", format!("{}", major_version))
            .env("PATH", path_env)
            .current_dir(shim_dir),
        &format!("shim for PG v{}", major_version),
    )?;

    if rc.status.code().unwrap() != 0 {
        panic!("failed to make pgx-cshim for v{}", major_version);
    }

    Ok(())
}

fn generate_common_rs(working_dir: PathBuf) {
    eprintln!("[all branches] Regenerating common.rs and XX_specific.rs files...");
    let cwd = std::env::current_dir().unwrap();

    std::env::set_current_dir(&working_dir).unwrap();
    let result = bindings_diff::main();
    std::env::set_current_dir(cwd).unwrap();

    if result.is_err() {
        panic!(result.err().unwrap());
    }
}

fn run_command(mut command: &mut Command, version: &str) -> Result<Output, std::io::Error> {
    let mut dbg = String::new();

    command = command
        .env_remove("DEBUG")
        .env_remove("MAKEFLAGS")
        .env_remove("MAKELEVEL")
        .env_remove("MFLAGS")
        .env_remove("DYLD_FALLBACK_LIBRARY_PATH")
        .env_remove("OPT_LEVEL")
        .env_remove("TARGET")
        .env_remove("PROFILE")
        .env_remove("OUT_DIR")
        .env_remove("HOST")
        .env_remove("NUM_JOBS");

    eprintln!("[{}] {:?}", version, command);
    dbg.push_str(&format!("[{}] -------- {:?} -------- \n", version, command));

    let output = command.output()?;
    let rc = output.clone();

    if !output.stdout.is_empty() {
        for line in String::from_utf8(output.stdout).unwrap().lines() {
            if line.starts_with("cargo:") {
                dbg.push_str(&format!("{}\n", line));
            } else {
                dbg.push_str(&format!("[{}] [stdout] {}\n", version, line));
            }
        }
    }

    if !output.stderr.is_empty() {
        for line in String::from_utf8(output.stderr).unwrap().lines() {
            dbg.push_str(&format!("[{}] [stderr] {}\n", version, line));
        }
    }
    dbg.push_str(&format!(
        "[{}] /----------------------------------------\n",
        version
    ));

    eprintln!("{}", dbg);
    Ok(rc)
}

fn apply_pg_guard(input: String) -> Result<String, std::io::Error> {
    let file = syn::parse_file(input.as_str()).unwrap();

    let mut stream = TokenStream2::new();
    for item in file.items.into_iter() {
        match item {
            Item::ForeignMod(block) => {
                stream.extend(quote! {
                    #[pg_guard]
                    #block
                });
            }
            _ => {
                stream.extend(quote! { #item });
            }
        }
    }

    Ok(format!("{}", stream.into_token_stream()))
}

fn rust_fmt(path: &str) -> Result<(), std::io::Error> {
    run_command(
        Command::new("rustfmt").arg(path).current_dir("."),
        "[bindings_diff]",
    )?;

    Ok(())
}

pub(crate) mod bindings_diff {
    use crate::rust_fmt;
    use elapsed::measure_time;
    use quote::{quote, ToTokens};
    use std::collections::HashSet;
    use std::io::{Read, Write};
    use std::path::PathBuf;
    use std::str::FromStr;

    pub(crate) fn main() -> Result<(), std::io::Error> {
        let mut v10 = read_source_file("src/pg10_bindings.rs");
        let mut v11 = read_source_file("src/pg11_bindings.rs");
        let mut v12 = read_source_file("src/pg12_bindings.rs");

        let mut versions = vec![&mut v10, &mut v11, &mut v12];
        let common = build_common_set(&mut versions);

        eprintln!(
            "[all branches]: common={}, v10={}, v11={}, v12={}",
            common.len(),
            v10.len(),
            v11.len(),
            v12.len(),
        );

        write_common_file("src/common.rs", common);
        write_source_file("src/pg10_specific.rs", v10);
        write_source_file("src/pg11_specific.rs", v11);
        write_source_file("src/pg12_specific.rs", v12);

        // delete the bindings files when we're done with them
        std::fs::remove_file(PathBuf::from_str("src/pg10_bindings.rs").unwrap())
            .expect("couldn't delete v10 bindings");
        std::fs::remove_file(PathBuf::from_str("src/pg11_bindings.rs").unwrap())
            .expect("couldn't delete v11 bindings");
        std::fs::remove_file(PathBuf::from_str("src/pg12_bindings.rs").unwrap())
            .expect("couldn't delete v12 bindings");

        Ok(())
    }

    fn build_common_set(versions: &mut Vec<&mut HashSet<String>>) -> HashSet<String> {
        let mut common = HashSet::new();

        for map in versions.iter() {
            for key in map.iter() {
                if !common.contains(key) && all_contain(&versions, &key) {
                    common.insert(key.clone());
                }
            }
        }

        for map in versions.iter_mut() {
            for key in common.iter() {
                map.remove(key);
            }
        }

        common
    }

    #[inline]
    fn all_contain(maps: &[&mut HashSet<String>], key: &String) -> bool {
        for map in maps.iter() {
            if !map.contains(key) {
                return false;
            }
        }

        true
    }

    fn read_source_file(filename: &str) -> HashSet<String> {
        let mut file = std::fs::File::open(filename).unwrap();
        let mut input = String::new();

        file.read_to_string(&mut input).unwrap();
        let source = syn::parse_file(input.as_str()).unwrap();

        let mut item_map = HashSet::new();
        for item in source.items.into_iter() {
            item_map.insert(item.to_token_stream().to_string());
        }

        item_map
    }

    fn write_source_file(filename: &str, items: HashSet<String>) {
        let mut file =
            std::fs::File::create(filename).expect(&format!("failed to create {}", filename));
        file.write_all(
            quote! {
                #![allow(clippy::all)]

                use crate as pg_sys;
                use pgx_macros::*;
                use crate::common::*;
            }
            .to_string()
            .as_bytes(),
        )
        .expect(&format!("failed to write to {}", filename));
        for item in items {
            file.write_all(item.as_bytes())
                .expect(&format!("failed to write to {}", filename));
        }
        rust_fmt(filename)
            .unwrap_or_else(|e| panic!("unable to run rustfmt for {}: {:?}", filename, e));
    }

    fn write_common_file(filename: &str, items: HashSet<String>) {
        let mut file = std::fs::File::create(filename).expect("failed to create common.rs");
        file.write_all(
            quote! {
                #![allow(clippy::all)]

                use crate as pg_sys;
                use pgx_macros::*;

                #[cfg(feature = "pg10")]
                use crate::pg10_specific::*;
                #[cfg(feature = "pg11")]
                use crate::pg11_specific::*;
                #[cfg(feature = "pg12")]
                use crate::pg12_specific::*;
            }
            .to_string()
            .as_bytes(),
        )
        .expect("failed to write to common.rs");

        for item in items.iter() {
            file.write_all(item.as_bytes())
                .expect("failed to write to common.rs");
        }
        rust_fmt(filename)
            .unwrap_or_else(|e| panic!("unable to run rustfmt for {}: {:?}", filename, e));
    }
}
