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


#[no_mangle]
fn my_guy() {
    panic!("this is plugin's my_guy()");
}
