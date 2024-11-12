fn main() {
    let integer = 1;
    let mut r = &integer;
    {
        let integer = 2;
        r = &integer
    }
    println!("{}", r);
}
