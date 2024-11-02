struct A;
fn consume(a: A) {}
fn borrow(a: &A) {}
fn test() {
    let a = A;
    consume(a);
    borrow(&a);
}
