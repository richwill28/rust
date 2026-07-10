// Test error when public function exposes private fields through views

mod inner {
    pub struct Point {
        pub x: i32,
        y: i32, // private
    }
}

use inner::Point;

// Error: public function exposes private field
pub fn expose_private(p: &{y} Point) {
    //~^ ERROR view cannot expose private field `y` in public context
}

// OK: public function uses public field
pub fn use_public(p: &{x} Point) {
}

// OK: private function can use private field
fn private_can_access(p: &{y} Point) {
}

fn main() {}
