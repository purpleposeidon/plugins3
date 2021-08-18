extern crate header;

use header::SayHelloService;

#[no_mangle]
pub extern "Rust" fn new_service() -> Box<dyn SayHelloService> {
    /*assert_eq!(header::get(), 1);
    header::set(2);*/
    Box::new(PluginSayHello::new())
}

pub struct PluginSayHello {
    id: String,
}

impl PluginSayHello {
    fn new() -> PluginSayHello {
        let id = format!("plugin");
        println!("[{}] Created instance!", id);
        PluginSayHello { id }
    }
}

impl SayHelloService for PluginSayHello {
    fn say_hello(&self) {
        println!("[{}] Hello from plugin!", self.id);
        header::greet();
    }
}

impl Drop for PluginSayHello {
    fn drop(&mut self) {
        println!("[{}] Destroyed instance!", self.id);
    }
}


// gcc -o libplugin.so -shared libplugin.rlib
// ld -shared -fPIC libplugin.rlib -o libplugin.so
//   empty
// gcc -Wl,--whole-archive -shared -fPIC -o libplugin.so ./libplugin.rlib
// cargo rustc --lib -p plugin -- --emit=obj
//  :|
//
//
// cargo rustc --lib -p plugin -- --emit=asm
//    edit out some *.py gdb directive lines, trim some stuff:
// 	.section	.debug_gdb_scripts,"aMS",@progbits,1,unique,1
// 	.section	.debug_gdb_scripts,"aMS",@progbits,1
//    See these URLs:
//        http://web.mit.edu/rhel-doc/3/rhel-as-en-3/section.html
//        https://sourceware.org/gdb/onlinedocs/gdb/dotdebug_005fgdb_005fscripts-section.html
// gcc -o ../libplugin.so -shared plugin-8af07f9ac7d7475c.s
//
// What about this
// $ cargo rustc -p plugin -- -C embed-bitcode=no -C prefer-dynamic=yes
// $ ~/tmp/clang+llvm-12.0.1-x86_64-linux-gnu-ubuntu-/bin/ld.lld -shared -o target/debug/libplugin.so target/debug/deps/plugin-8af07f9ac7d7475c.o

