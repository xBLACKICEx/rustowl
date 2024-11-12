fn main() {
    let mut string = String::from("A");
    let reference = &string;
    string.push_str("B");
    println!("{}", reference);
    string.into_bytes();
    println!("{}", string);
}
