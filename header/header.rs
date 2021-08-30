pub trait SayHelloService {
    fn say_hello(&self);
}

pub fn greet() {
    println!("FORBID_ME");
    private();
}

fn private() {
    println!("this is a private symbol");
}

pub fn unused_public() {
    println!("this is a publi symbol that nobody calls; but it should still be visible");
}

#[allow(dead_code)]
fn private_uncalled() {
    println!("this is a private symbol; it might disappear");
}


static mut GLOBAL: i32 = 0;
pub fn get() -> i32 { unsafe { GLOBAL } }
pub fn set(v: i32) { unsafe { GLOBAL = v; } }
