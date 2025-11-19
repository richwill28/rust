// Test view syntax in `impl` methods
//@ check-pass

struct Point {
    x: i32,
    y: i32,
}

// In impl methods
impl Point {
    fn method(&{x} self) -> &i32 {
        &self.x
    }
    
    fn method_mut(&mut {mut x} self) -> &mut i32 {
        &mut self.x
    }
}

fn main() {}
