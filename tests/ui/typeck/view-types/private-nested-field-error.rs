// Test error when nested field path includes private fields in public function

mod inner {
    pub struct Outer {
        pub public_field: Inner,
        private_field: Inner,
    }
    
    pub struct Inner {
        pub public_value: i32,
        private_value: i32,
    }
}

use inner::Outer;

// Error: accessing private nested field
pub fn private_in_nested(o: &{public_field.private_value} Outer) {
    //~^ ERROR view cannot expose private field `private_value` in public context
}

// Error: accessing field on private parent
pub fn private_parent_field(o: &{private_field.public_value} Outer) {
    //~^ ERROR view cannot expose private field `private_field` in public context
}

// OK: all public fields
pub fn all_public(o: &{public_field.public_value} Outer) {
}

fn main() {}
