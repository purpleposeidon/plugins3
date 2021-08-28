extern crate header; // FIXME
use header::SayHelloService;

use std::path::*;
use std::ffi::OsStr;
#[allow(unused_imports)]
use std::process::Command;

fn glob1(dir: &Path, prefix: &str, suffix: &str) -> Option<PathBuf> {
    let mut found = None;
    for entry in dir.read_dir().ok()? {
        if let Ok(entry) = entry {
            let entry = entry.path();
            let is_so = entry.extension() == Some(OsStr::new(suffix));
            let is_std = entry.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(prefix)) == Some(true);
            if is_so && is_std {
                if found.is_some() {
                    panic!("multiple {}*.{}'s found in {:?}", prefix, suffix, dir);
                }
                found = Some(entry);
            }
        }
    }
    found
}

#[cfg(target_os = "linux")]
fn find_rustc_std() -> Option<PathBuf> {
    let mut stdpath = Command::new("rustc");
    let stdpath = stdpath.arg("--print").arg("sysroot");
    let stdpath = stdpath.output().ok()?;
    let stdpath = std::str::from_utf8(&stdpath.stdout).expect("find_std parse");
    if stdpath.is_empty() { return None; }
    let stdpath = stdpath.strip_suffix('\n').expect("strip");
    let stdpath = format!("{}/lib/", stdpath);
    let stdpath = std::path::Path::new(&stdpath);
    glob1(&stdpath, "libstd-", "dll")
}

#[cfg(target_os = "windows")]
fn find_rustc_std() -> Option<PathBuf> {
    // FIXME: Implement
    None
}

fn seek(path: String) -> Option<PathBuf> {
    let pb: PathBuf = path.into();
    if pb.exists() {
        Some(pb)
    } else {
        None
    }
}

fn seek_lib(name: &str) -> Result<PathBuf, String> {
    None
        .or_else(|| seek(format!("./target/x86_64-pc-windows-msvc/debug/{}", name)))
        .or_else(|| seek(format!("./{}", name)))
        .or_else(|| seek(format!("./lib/{}", name)))
        .ok_or_else(|| format!("Unable to find {}", name))
}

fn main() {
    /*{
        assert_eq!(header::get(), 0);
        header::set(1);
        assert_eq!(header::get(), 1);
    }*/
    unsafe {
        // NOTE: If running under wine, you may need to put vcruntime140d.dll by the .exe
        // if vcruntime isn't linked statically.
        let _std = if let Some(std) = find_rustc_std() {
            libloading::Library::new(&std).expect("load rustup std")
        } else {
            let std = seek(format!("std.dll"))
                .or_else(|| glob1(Path::new("./msvc_rust/"), "std-", "dll"))
                .expect("find std.dll");
            libloading::Library::new(&std).expect("load nearby std")
        };
        let _header = libloading::Library::new(seek_lib("header.dll").unwrap()).expect("load header.dll");
        let plugin = libloading::Library::new(seek_lib("plugin.dll").unwrap()).expect("load plugin.dll");

        type F = extern "Rust" fn() -> Box<dyn SayHelloService>;
        println!("Okay, that's a start!");
        let new_service: libloading::Symbol<F> = plugin.get(b"new_service").expect("load symbol");
        let service = new_service();
        service.say_hello();
        println!("Hooray!");
    }
    /*{
        assert_eq!(header::get(), 2);
    }*/
}
