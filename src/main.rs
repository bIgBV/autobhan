use monza::Buffer;

fn main() {
    let buf = Buffer::new(10);

    buf.push(10);
    assert_eq!(buf.pop(), Some(&10));
}
