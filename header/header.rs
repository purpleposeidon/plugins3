pub trait SayHelloService {
    fn say_hello(&self);
}

pub fn greet() {
    println!("FORBID_ME");
}


static mut GLOBAL: i32 = 0;
pub fn get() -> i32 { unsafe { GLOBAL } }
pub fn set(v: i32) { unsafe { GLOBAL = v; } }
