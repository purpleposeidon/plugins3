extern crate header;
extern crate glob;
#[allow(unused_imports)] #[macro_use] extern crate eztrace;

use header::SayHelloService;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write as _};
use std::path::*;
use std::process::Command;
use std::process::Stdio;
use std::time::{Instant, SystemTime};

type Triple = &'static str;
const TRIPLE_LINUX: Triple = "x86_64-unknown-linux-gnu";
const TRIPLE_WINDOWS: Triple = "x86_64-pc-windows-msvc";
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct Pair {
    host: Triple,
    target: Triple,
}
impl Pair {
    fn target(&self) -> String {
        if self.host == self.target {
            format!("./target/{}", PROFILE)
        } else {
            format!("./target/{}/{}", self.target, PROFILE)
        }
    }
    fn libname(&self, package: &str) -> String {
        match self.target {
            TRIPLE_LINUX => format!("lib{}.so", package),
            TRIPLE_WINDOWS => format!("{}.dll", package),
            e => todo!("libname {:?}", e), // This'll break if mac/bsd get added; their dylib names are the same as linux's
        }
    }
    fn foreign(&self) -> bool { self.host != self.target }
}

#[cfg(target_os = "linux")]
const HOST: Triple = TRIPLE_LINUX;
#[cfg(target_os = "windows")]
const HOST: Triple = TRIPLE_WINDOWS;

const DEBUG: bool = cfg!(debug_assertions);
const RELEASE: bool = !DEBUG;
const PROFILE: &'static str = if DEBUG {
    "debug"
} else {
    "release"
};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct Config {
    host: String,
    target: String,
    cmd: String,
}

#[derive(Debug)]
struct Toolchain {
    compile: bool,
    cmds: HashMap<Config, Vec<String>>,
}
impl Toolchain {
    fn load() -> Option<Self> {
        let fd = File::open("./toolchain.txt").ok()?;
        let fd = BufReader::new(fd);
        let mut ret = Toolchain {
            compile: std::env::args().find(|a| a == "--compile").is_some(),
            cmds: Default::default(),
        };

        for line in fd.lines() {
            let line = line.expect("readline");
            let line = line.trim();
            if line.starts_with('#') { continue; }
            if line.is_empty() { continue; }
            let mut line = line.split_whitespace();
            let host = line.next();
            let target = line.next();
            let cmd = line.next();
            let args = line
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            fn eat(t: Option<&str>, prefix: &str) -> String {
                let t = t.unwrap_or_else(|| panic!("expected something starting with {:?}", prefix));
                assert!(t.starts_with(prefix), "expected something starting with {:?}", prefix);
                (&t[prefix.len()..]).to_string()
            }
            let host = eat(host, "host:");
            let target = eat(target, "target:");
            let cmd = eat(cmd, "cmd:");
            ret.cmds.insert(
                Config { host, target, cmd },
                args,
            );
        }
        Some(ret)
    }
    fn get(&self, pair: Pair, cmd: &str, env: &[(&str, String)]) -> Command {
        let cfg = Config {
            host: pair.host.to_string(),
            target: pair.target.to_string(),
            cmd: cmd.to_string(),
        };
        let cmds = self.cmds
            .get(&cfg)
            .unwrap_or_else(|| panic!("Command for doing {:?} not provided in toolchain.txt", cfg));
        let mut cmds: Vec<String> = cmds.clone();
        for c in &mut cmds {
            for (k, v) in env {
                *c = c.replace(k, &v);
            }
        }
        cmds.retain(|c| !c.is_empty());
        for c in &cmds {
            if c.contains('$') {
                panic!("{:?} has unexpanded variables", cmds);
            }
        }
        let mut globbed = vec![];
        for c in &cmds {
            if cfg!(target_os = "windows") {
                // Windows commands handle their own globbing.
                globbed.push(c.clone());
            } else if c.contains('*') {
                let mut any = false;
                for g in glob::glob(c).expect("bad glob") {
                    any = true;
                    let g = g.expect("expand glob");
                    let g = g.as_os_str();
                    let g = g.to_str().unwrap_or_else(|| panic!("dirty string expanded from glob {:?}", g));
                    globbed.push(g.to_string());
                }
                if !any {
                    globbed.push(c.clone());
                }
            } else {
                globbed.push(c.clone());
            }
        }
        let mut cmd = Command::new(&globbed[0]);
        cmd.args(&globbed[1..]);
        cmd
    }
}


fn glob1(dir: &Path, prefix: &str, suffix: &str) -> Option<PathBuf> {
    assert!(!suffix.starts_with('.'));
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

fn find_rust_std(toolchain: &Toolchain, pair: Pair, token_package: &str) -> Option<PathBuf> {
    let mut stdpath = toolchain.get(pair, "cargo", &[]);
    stdpath
        .arg("--quiet")
        .args(&["rustc", "-p", token_package]);
    if pair.foreign() {
        stdpath.arg(format!("--target={}", pair.target));
    }
    stdpath
        .arg("--")
        .args(&["--print", "sysroot"]);
    let stdpath = stdpath.output().ok()?;
    let stdpath = std::str::from_utf8(&stdpath.stdout).expect("find_rust_std parse");
    if stdpath.is_empty() { return None; }
    let stdpath = stdpath.strip_suffix('\n').expect("strip");
    let stdpath = format!("{}/lib/", stdpath);
    let mut stdpath: PathBuf = stdpath.into();
    if pair.foreign() {
        // lib/rustlib/x86_64-pc-windows-msvc/lib/std-3d786a338e3fbd3c.dll.lib
        // (Windows uses the same structure)
        stdpath.push("rustlib");
        stdpath.push(pair.target);
        stdpath.push("lib");
    }
    match pair.target {
        TRIPLE_LINUX => glob1(&stdpath, "libstd-", "so"),
        TRIPLE_WINDOWS => glob1(&stdpath, "std-", "dll"),
        e => todo!("find_rust_std {:?}", e),
    }
}

fn seek(path: String) -> Option<PathBuf> {
    let pb: PathBuf = path.into();
    if pb.exists() {
        Some(pb)
    } else {
        None
    }
}

fn seek_lib(pair: Pair, package: &str) -> Result<PathBuf, String> {
    let mut hay = String::new();
    macro_rules! seek {
        ($($tt:tt)*) => {
            let path = format!($($tt)*);
            write!(hay, "\n  {}", path).ok();
            if let Some(path) = seek(path) {
                return Ok(path);
            }
        };
    }
    let libname = pair.libname(package);
    seek!("./target/{}/{}/{}", pair.target, PROFILE, libname);
    seek!("./target/{}/{}", PROFILE, libname);
    seek!("./lib/{}", libname);
    seek!("./{}/lib/{}", package, libname);
    seek!("./{}", libname);
    Err(format!("Unable to find {:?}.\nLooked in:{}\n", libname, hay))
}

fn modified(path: &Path) -> Option<SystemTime> {
    match path.metadata() {
        Ok(md) => match md.modified() {
            Ok(md) => Some(md),
            Err(e) => panic!("(1)unable to get modification time of {:?}: {}", path, e),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => panic!("(2)unable to get modification time of {:?}: {}", path, e),
    }
}

fn dirty(
    inputs: &str,
    output: &str,
) -> bool {
    let output = Path::new(output);
    let output = if let Some(output) = modified(output) {
        output
    } else {
        return true;
    };
    let mut any = false;
    for g in glob::glob(&inputs).expect("bad glob string") {
        any = true;
        let g = g.expect("expand glob");
        if let Some(g) = modified(&g) {
            if g > output {
                return true;
            }
        } else {
            return true;
        }
    }
    if !any {
        panic!("no input files {:?}", inputs);
    }
    false
}

struct Lib {
    name: &'static str,
    has_exports: bool,
    dependencies: &'static [&'static str],
}

fn compile_dylib(
    toolchain: &Toolchain,
    pair: Pair,
    package: Lib,
) -> PathBuf {
    let std = find_rust_std(toolchain, pair, package.name).expect("failed to find rust std");
    let std = std.to_str().expect("bad utf8 in std path");
    // Assumes the command only modifies the .o files if the source hasn't changed.
    let mut cmd = toolchain.get(pair, "cargo", &[]);
    cmd.arg("rustc");
    if pair.foreign() {
        cmd.arg(format!("--target={}", pair.target));
    }
    if RELEASE {
        cmd.arg("--release");
    }
    cmd.args(["-p", package.name, "--", "--emit=obj"]);
    let start = Instant::now();
    assert!(cmd.status().unwrap().success());

    let libname = pair.libname(package.name);
    let objects = format!("{}/deps/{}-*.o", pair.target(), package.name);
    if !dirty(&objects, &libname) {
        println!("  {:?} (clean)", start.elapsed());
        return libname.into();
    }
    let mut env = vec![];

    let dll_export: String;
    let target_out = pair.target();
    if pair.target == TRIPLE_WINDOWS && package.has_exports {
        // Collect the exported symbols.
        let mut dis = toolchain.get(pair, "llvm-dis", &[
            ("$OBJECTS", objects.clone()),
        ]);
        dis.stdout(Stdio::piped());
        let dis = dis.spawn().expect("failed to spawn llvm-dis");
        let out = BufReader::new(dis.stdout.expect("llvm-dis stdout"));
        dll_export = format!("{}/deps/{}.dll_export", target_out, package.name);
        let linkage_names = File::create(dll_export.clone());
        let linkage_names = linkage_names
            .unwrap_or_else(|e| panic!("create dll_export at {:?}: {}", dll_export, e));
        let mut linkage_names = BufWriter::new(linkage_names);
        // _ZN6header3set17h7991ffbe918cc6e2E
        let this_crate = format!("_ZN{}{}", package.name.len(), package.name);
        'next_line: for line in out.lines() {
            // !159 = distinct !DISubprogram(name: "greet", linkageName: "_ZN6header5greet17hdd2049c368ec70b1E", scope: !161, file: !160, line: 5, type: !162, scopeLine: 5, flags: DIFlagPrototyped, spFlags: DISPFlagDefinition, unit: !3, templateParams: !66, retainedNodes: !66)
            let line = line.expect("reading output of llvm-dis failed");
            let mut line = line.split(&[' ', ':', ',', '(', ')'][..]);
            macro_rules! next {
                () => { if let Some(l) = line.next() { l } else { continue 'next_line; } };
            }
            macro_rules! find {
                ($t:literal) => {
                    loop {
                        let l = next!();
                        if l == $t { break; }
                    }
                };
            }
            if !next!().starts_with('!') { continue 'next_line; }
            find!("!DISubprogram");
            find!("linkageName");
            let linkage_name = line.filter(|x| !x.is_empty()).next().expect("linkageName argument");
            let linkage_name = linkage_name.replace('"', "");
            if !linkage_name.starts_with(&this_crate) { continue; }
            write!(linkage_names, "/export:{}\r\n", linkage_name).expect("write dll_export");
        }
        linkage_names.flush().expect("flush linkage_names");
        {linkage_names};
        env.push(("$EXPORTS_LIST", format!("@{}", dll_export)));
    } else if pair.target == TRIPLE_WINDOWS {
        env.push(("$EXPORTS_LIST", String::new()));
    }
    let lib_out = format!("{}/{}", target_out, libname);
    env.push(("$STD", std.into()));
    env.push(("$OUT", lib_out.clone()));
    env.push(("$INPUT_OBJ", objects));
    {
        let mut lib_deps = String::new();
        let mut first = true;
        for lib in package.dependencies {
            if first {
                first = false;
            } else {
                write!(lib_deps, " ").unwrap();
            }
            write!(lib_deps, "/defaultlib:{}/{}.lib", pair.target(), lib).unwrap();
        }
        env.push(("$DLL_LIB_DEPENDENCIES", lib_deps));
    }
    let mut link = toolchain.get(pair, "link", &env[..]);
    // $ "./lld-link-12.exe" "/dll" "/noentry" "@./target/x86_64-pc-windows-msvc/debug/deps/plugin.dll_export" "/out:./target/x86_64-pc-windows-msvc/debug/plugin.dll" "/defaultlib:./msvc_vc_lib/msvcurtd.lib" "/defaultlib:/home/poseidon/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-pc-windows-msvc/lib/std-3d786a338e3fbd3c.dll.lib" "/defaultlib:./target/x86_64-pc-windows-msvc/debug/header.lib" "target/x86_64-pc-windows-msvc/debug/deps/plugin-ecc185708dca4430.o" 
    // "/defaultlib:./target/x86_64-pc-windows-msvc/debug/header.lib"
    trace!(link);
    assert!(link.status().unwrap().success(), "link failed");
    let lib_out: PathBuf = lib_out.into();
    if package.name != "header" {
        assert_clean(&lib_out);
    }
    println!("  {:?}", start.elapsed());
    lib_out
}

fn assert_clean(plugin: &Path) {
    let name = plugin.display();
    let mut buf = vec![];
    let mut plugin = std::fs::File::open(plugin)
        .unwrap_or_else(|e| panic!("unable to assert_clean() {:?}: {}", plugin, e));
    plugin.read_to_end(&mut buf).expect("read failed");
    let buf = String::from_utf8_lossy(&buf);
    let mut bad = format!("{}_{}", "forbid", "me");
    // If "FORBID_ME" occurs in libplugin.so, the full contents are being linked in.
    bad.make_ascii_uppercase();
    assert!(!buf.contains(&bad), "libplugin.so contains the fobidden test string");
    println!("{} looks clean.", name);
}

fn use_plugin(plugin: &libloading::Library) {
    assert_eq!(header::get(), 0);
    header::set(1);
    assert_eq!(header::get(), 1);
    println!("Okay, that's a start!");
    type F = extern "Rust" fn() -> Box<dyn SayHelloService>;
    let new_service: libloading::Symbol<F> = unsafe { plugin.get(b"new_service").expect("load symbol") };
    let service = new_service();
    service.say_hello();
    /*assert_eq!(header::get(), 2);*/
    println!("Hooray!");
}

fn main() {
    let native = Pair {
        host: HOST,
        target: HOST,
    };
    let (std, header, plugin) = if let Some(ref toolchain) = Toolchain::load() {
        let std = find_rust_std(toolchain, native, "header").expect("failed to find rust std");

        if toolchain.compile {
            // FIXME: Copy in libstd.so, std.dll, std.dll.lib, msvrt.{lib,dll}
        }
        let mut pairs = vec![native];
        if toolchain.compile {
            pairs.push(Pair {
                host: HOST,
                target: if HOST == TRIPLE_LINUX {
                    TRIPLE_WINDOWS
                } else {
                    TRIPLE_LINUX
                },
            });
        }
        println!("compiling plugins...");
        let mut hp = None;
        for &pair in pairs.iter().rev() {
            hp = Some((
                compile_dylib(toolchain, pair, Lib {
                    name: "header",
                    has_exports: true,
                    dependencies: &[],
                }),
                compile_dylib(toolchain, pair, Lib {
                    name: "plugin",
                    has_exports: false,
                    dependencies: &["header"],
                }),
            ));
        }
        println!("done");
        if toolchain.compile { return; }
        let (h, p) = hp.unwrap();
        (std, h, p)
    } else {
        (
            seek_lib(native, &native.libname("std")).unwrap(),
            seek_lib(native, &native.libname("header")).unwrap(),
            seek_lib(native, &native.libname("plugin")).unwrap(),
        )
    };
    #[cfg(target_os = "linux")]
    unsafe {
        pub use libloading::os::unix::*;
        let _std = Library::open(Some(&std), RTLD_GLOBAL | RTLD_NOW).expect("load std");
        let _header = Library::open(Some(header), RTLD_GLOBAL | RTLD_NOW).expect("load header");
        let plugin = libloading::Library::new(plugin).expect("load plugin");
        use_plugin(&plugin);
    }
    #[cfg(target_os = "windows")]
    unsafe {
        // NOTE: If running under wine, you may need to put vcruntime140d.dll by the .exe
        // if vcruntime isn't linked statically.
        let _std = libloading::Library::new(&std).expect("load std.dll");
        let _header = libloading::Library::new(header).expect("load header.dll");
        let plugin = libloading::Library::new(plugin).expect("load plugin.dll");
        use_plugin(&plugin);
    }
}
