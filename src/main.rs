use monza::Buffer;

fn main() {
    let buf = Buffer::new(10);

    buf.insert(10);
    assert_eq!(buf.get(), Some(&10));
}
