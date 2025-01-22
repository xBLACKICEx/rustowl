
fn main() {
    let mut s = String::from("hello");
    let r = &s;
    s.push_str("world");
    println!("{}", &r);
}


use std::sync::{Arc, Mutex};

fn mutex() {
    let m = Arc::new(Mutex::new(1));
    let m1 = m.clone();
    let m2 = m.clone();
    if let Ok(locked) = m.lock() {
        m1.lock();
    } else {
        m2.lock();
    }
}

struct S<'a, T> (&'a T);

impl<'a, T> Drop for S<'a, T> {
    fn drop(&mut self) { todo!() }
}

fn main() {
    let mut x = 10;
    let _y = S(&x);
    //drop(_y);
    let _z = &mut x;
}
