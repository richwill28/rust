// Test that views work in impl methods
//@ check-pass

struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn get_x(&{x} self) -> &i32 {
        &self.x
    }
    
    fn get_both(&{x, y} self) -> (&i32, &i32) {
        (&self.x, &self.y)
    }
    
    fn modify_x(&mut {mut x} self, new_x: i32) {
        self.x = new_x;
    }
}

fn main() {
    let mut p = Point { x: 10, y: 20 };
    let _ = p.get_x();
    let _ = p.get_both();
    let _ = p.modify_x(30);
}
