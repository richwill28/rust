// Test that pub(crate) functions can access private fields
//@ check-pass

mod inner {
    pub struct Point {
        pub x: i32,
        y: i32, // private
    }
    
    // pub(crate) can access private fields
    pub(crate) fn access_private_crate(p: &{y} Point) -> &i32 {
        &p.y
    }
    
    // pub(super) can access private fields
    pub(super) fn access_private_super(p: &{y} Point) -> &i32 {
        &p.y
    }
    
    // private function can access private fields
    fn access_private(p: &{y} Point) -> &i32 {
        &p.y
    }
}

fn main() {}
